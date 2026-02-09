# Decoding Parameters - Constat actuel

## Valeurs par defaut (identiques pour les deux backends)

| Parametre | Valeur | Fichier |
|-----------|--------|---------|
| `beam_width` | 1 (greedy) | Rust `src/engine/config.rs:17`, TS `src/lib/types.ts:29`, Swift `sidecar/.../main.swift:23` |
| `temperature` | 1.0 | Rust `src/engine/config.rs:18`, TS `src/lib/types.ts:30`, Swift `sidecar/.../main.swift:24` |
| `blank_penalty` | 6.0 | Rust `src/engine/config.rs:19`, TS `src/lib/types.ts:31`, Swift `sidecar/.../main.swift:25` |

Aucune differenciation par backend. Le meme `DecodingConfig` est passe tel quel a OpenVINO et CoreML.

## Valeurs cibles (profil qualite)

| Parametre | OpenVINO (Intel) | CoreML (Mac Silicon) | Pourquoi |
|-----------|-----------------|---------------------|----------|
| `blank_penalty` | 11.0-12.0 | 10.5-11.5 | Empeche le modele de sauter des mots complexes ("premier", "troisieme") |
| `beam_width` | 5-6 | 4-5 | Equilibre precision/vitesse. Apple Silicon gere bien un faisceau de 5 |
| `temperature` | 0.6 | 0.5 | Evite les hallucinations ("Mm-S-E") tout en restant souple pour le francais |

## Probleme CoreML

Le sidecar actif (`@main ParakeetCoreML`, main.swift:90-138) utilise **FluidAudio `AsrManager`** avec `config: .default`. Il **ignore completement** les parametres `beamWidth`, `temperature` et `blankPenalty` passes en CLI.

Ces parametres ne sont lus que par le decoder custom commente (main.swift:145+, `TDTDecoder`).

Consequence : pour CoreML, les valeurs de decodage n'ont actuellement **aucun effet**.

## Actions a mener

1. **Implementer des defaults par backend** dans `DecodingConfig` (Rust) et `DEFAULT_TRANSCRIPTION_SETTINGS` (TS)
2. **Corriger le sidecar CoreML** pour propager les parametres a FluidAudio, ou reactiver le decoder custom
3. **Benchmarker** les valeurs cibles sur des echantillons francais avant de les figer comme defaults
