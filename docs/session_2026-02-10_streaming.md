# Session 10 février 2026 — Transcription streaming incrémentale

## Objectif

Implémenter la transcription incrémentale en mode dictée : au lieu d'attendre le clic Stop pour transcrire tout l'audio d'un bloc, envoyer des chunks de ~10-15s au moteur pendant l'enregistrement et afficher le texte au fur et à mesure.

## Ce qui a été implémenté

### 1. Pipeline de streaming (6 fichiers modifiés)

**Backend Rust :**

- **`audio/capture.rs`** — Ajout de `DrainChunk` dans l'enum `AudioCommand` et méthode `drain_chunk(max_samples)` pour extraire l'audio accumulé sans arrêter l'enregistrement
- **`engine/mod.rs`** — Ajout de `transcribe_chunk()` sur `DynamicEngine` (wrapper léger qui appelle `run_inference` et retourne juste le texte)
- **`commands/transcription.rs`** — Nouveaux états partagés (`StreamingState`, `StreamingDone`, `StreamingSegments`), nouvelle commande async `start_streaming_transcription` (boucle 10s), modification de `stop_recording` (async, synchronisation via Notify)
- **`lib.rs`** — Enregistrement des nouveaux managed states et commande

**Frontend TS :**

- **`src/lib/tauri.ts`** — Wrapper `startStreamingTranscription()`
- **`src/hooks/useRecording.ts`** — Appel fire-and-forget dans `start()`, fix du listener (useRef pour elapsedMs)

### 2. Qualité audio des chunks

- **VAD-based cutting** — Utilisation de `find_best_cut_point()` pour couper dans les silences plutôt qu'aveuglément toutes les 10s. Recherche dans les 3 dernières secondes du chunk
- **Normalisation séparée du resample** — Le resample se fait au drain, la normalisation juste avant l'inférence sur le chunk complet (gain cohérent, pas de discontinuité aux frontières leftover/nouveau)
- **Leftover management** — L'audio résiduel après la coupure VAD est conservé en domaine 16kHz brut et préfixé au prochain cycle

### 3. Filtrage des hallucinations (`engine/mod.rs`)

- **Leading filters** — Regex pour ponctuation parasite et mots courts en début de chunk
- **Repetition filter** — Détection de boucles n-gram (2-6 mots répétés 3+ fois), troncature récursive
- **Trailing filter** — Suppression conservatrice de fragments < 15 chars après la dernière ponctuation de fin de phrase. Language-agnostic (pas de détection ASCII)

### 4. Protections TDT anti-boucle décodeur (`engine/parakeet.rs`)

Trois protections alignées sur FluidAudio `TdtDecoderV3.swift`, implémentées dans `tdt_greedy_decode` ET `tdt_beam_decode` :

| Protection | Constante | Comportement |
|---|---|---|
| **non-blank dur=0** | — | Si un token non-blank a dur=0 et qu'un token a déjà été émis à cette même frame → forcer dur=1 (le premier dur=0 est autorisé, le deuxième est bloqué) |
| **maxSymbolsPerStep** | `MAX_SYMBOLS_PER_STEP = 10` | Compteur d'émissions par frame. Si ≥ 10, force l'avancement de t d'au moins 1 frame |
| **maxTokensPerChunk** | `MAX_TOKENS_PER_CHUNK = 150` | Arrêt précoce si le décodeur émet trop de tokens pour un chunk de 15s max |

`BeamHypothesis` enrichi avec `last_emission_time` et `emissions_at_time` pour le tracking per-beam.

## Analyse des problèmes rencontrés

### CoreML
Fonctionne bien avec le streaming. Pas de problème de boucle décodeur car FluidAudio (utilisé via sidecar) a déjà toutes ces protections.

### OpenVINO — problèmes identifiés et corrigés

1. **Chunks passant en anglais malgré forçage français** — Causé par le leftover silencieux (0.95s, RMS très faible) préfixé au chunk suivant. La frame encodeur t=0 est quasi-silence, le décodeur hallucine en anglais. Correction : les protections dur=0 empêchent la dérive.

2. **Boucle décodeur infinie (dur=0)** — Tokens non-blank émis avec dur=0 en cascade : le temps ne progresse jamais, le décodeur boucle sur la même frame. Corrigé par les 3 protections ci-dessus.

3. **Hallucinations résiduelles** — Ponctuation parasite en début de chunk, fragments courts en fin, boucles de répétition. Corrigé par les filtres dans `filter_chunk_hallucinations()`.

## Fichiers clés modifiés

```
src-tauri/src/audio/capture.rs          — DrainChunk command
src-tauri/src/engine/mod.rs             — transcribe_chunk(), hallucination filters
src-tauri/src/engine/parakeet.rs        — 3 protections TDT anti-boucle
src-tauri/src/commands/transcription.rs — streaming loop, stop_recording async
src-tauri/src/lib.rs                    — managed states
src/lib/tauri.ts                        — startStreamingTranscription wrapper
src/hooks/useRecording.ts               — fire-and-forget + listener fix
```

## État actuel

- **Streaming fonctionne** : le texte apparaît toutes les ~10s pendant l'enregistrement
- **Stop produit la transcription finale** : segments accumulés + queue restante, sauvegardé en DB
- **CoreML** : résultats corrects
- **OpenVINO** : amélioré avec les protections TDT, à re-tester pour confirmer

## Prochaines étapes possibles

- **Tester OpenVINO** avec les 3 nouvelles protections TDT — vérifier que les boucles décodeur et le passage en anglais sont résolus
- **Filtrer le leftover silencieux** (correction B proposée mais non implémentée) — jeter le leftover si RMS < seuil plutôt que le préfixer au chunk suivant
- **Ajuster le timing des segments** — les timestamps `startMs`/`endMs` des segments streaming sont approximatifs (basés sur la durée audio, pas sur les timestamps encoder)
- Refactoring listés dans `docs/todo_clear_engine.md`
