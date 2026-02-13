# Session 12 février 2026 — Analyse modèles CoreML et stratégie de distribution

## Objectif

Identifier précisément les fichiers modèles nécessaires par backend en vue de la distribution de l'app définitive (initialement via ZIP S3, finalement abandonnée au profit du bundling direct).

## Analyse du répertoire model/coreml/

### Fichiers présents (10 modèles .mlmodelc + fichiers annexes)

Le répertoire contenait des modèles utilisés et inutilisés. Après analyse du code (`main.swift`, `coreml.rs`), les fichiers non référencés ont été déplacés dans `model/coreml_notuse/` :

**Conservés dans `model/coreml/`** (utilisés par le custom decoder commenté) :
- `Preprocessor.mlmodelc` (520 Ko) — mel spectrogram
- `Encoder.mlmodelc` (425 Mo) — FastConformer encoder
- `Decoder.mlmodelc` (23 Mo) — LSTM decoder
- `RNNTJoint.mlmodelc` (12 Mo) — joint network
- `Melspectrogram_15s.mlmodelc` (620 Ko) — fallback preprocessor
- `ParakeetEncoder_15s.mlmodelc` (425 Mo) — fallback encoder
- `parakeet_v3_vocab.json` (148 Ko) — vocabulaire

**Déplacés dans `model/coreml_notuse/`** (non référencés) :
- `JointDecision.mlmodelc`, `JointDecisionv2.mlmodelc`, `MelEncoder.mlmodelc`, `ParakeetDecoder.mlmodelc`
- `parakeet_vocab.json` (doublon ancien), `config.json` (vide), `mlpackages/` (sources)

## Fonctionnement de FluidAudio (IMPORTANT)

### Qu'est-ce que FluidAudio ?

Librairie Swift open-source ([FluidInference/FluidAudio](https://github.com/FluidInference/FluidAudio), v0.12.0) qui fournit une API haut niveau pour la transcription vocale via CoreML. Utilisée dans le sidecar `parakeet-coreml`.

### Comment elle gère les modèles

```swift
let models = try await AsrModels.downloadAndLoad(version: .v3)
let asr = AsrManager(config: .default)
let result = try await asr.transcribe(audioURL, source: .system)
```

FluidAudio **télécharge et gère ses propres modèles** indépendamment :
- **Premier lancement** : télécharge les modèles Parakeet v3 depuis internet et les cache localement
- **Lancements suivants** : utilise le cache local, **pas de re-téléchargement**
- **Pendant une session** : les modèles restent en mémoire (sidecar persistent)

### Conséquence clé

Les fichiers dans `model/coreml/` ne sont **PAS utilisés** par le code actif. FluidAudio utilise ses propres modèles téléchargés. Le répertoire `model/coreml/` n'existe que pour le custom decoder (commenté).

### Avantages de FluidAudio
- API simple (une ligne pour transcrire)
- Décodeur TDT mature et testé (les protections anti-boucle de `parakeet.rs` sont calquées sur `TdtDecoderV3.swift` de FluidAudio)
- Gestion automatique du téléchargement et du cache des modèles

### Limitations de FluidAudio
- **Boîte noire** : les paramètres `temperature`, `blank_penalty`, `beam_width`, `language` envoyés par Rust sont ignorés
- **Nécessite internet** au premier lancement (téléchargement des modèles)
- **Pas de parité** garantie avec le backend OpenVINO (comportement différent, pas de réglage possible)

## Test du custom decoder (abandonné)

Tentative de réactivation du custom decoder Swift (code commenté dans `main.swift`) pour remplacer FluidAudio. Deux bugs identifiés et corrigés dans le code commenté :

1. **Duration +1** : `decodeLogits` retournait `maxDur + 1` au lieu de `maxDur` (le bin index EST la durée, pas +1). Causait un décodage trop rapide, perte de contenu.
2. **Pas de protections anti-boucle** : ajout de dur=0 protection, maxSymbolsPerStep=10, maxTokensPerChunk=150 (aligné sur `parakeet.rs`).

Malgré les corrections, la qualité de transcription restait inférieure à FluidAudio. **Décision : conserver FluidAudio** comme backend CoreML.

Les corrections restent dans le code commenté pour référence future.

## Stratégie de distribution — décision finale

L'idée initiale de distribuer les modèles via ZIP sur S3 est **abandonnée**. Les modèles sont intégrés directement dans l'app buildée.

### Ce qui est bundlé dans l'app

**Uniquement `model/openvino/`** — 9 fichiers, ~1.1 Go :

| Fichier | Taille |
|---|---|
| `parakeet_encoder.xml` + `.bin` | 2 Mo + 1.1 Go |
| `parakeet_decoder.xml` + `.bin` | 35 Ko + 23 Mo |
| `parakeet_joint.xml` + `.bin` | 13 Ko + 12 Mo |
| `parakeet_melspectogram.xml` + `.bin` | 48 Ko + 466 Ko |
| `parakeet_v3_vocab.json` | 156 Ko |

### Comportement par plateforme

| Plateforme | Backend auto-détecté | Source des modèles | Offline au 1er lancement ? |
|---|---|---|---|
| macOS ARM (Apple Silicon) | CoreML | FluidAudio télécharge + cache | **Non** (internet requis 1 fois) |
| macOS Intel | OpenVINO | Bundlé dans l'app | Oui |
| Windows | OpenVINO | Bundlé dans l'app | Oui |

## Fichiers modifiés cette session

Aucune modification persistante — tous les changements (activation/désactivation du custom decoder) ont été revertés. L'état du code est identique au début de la session, à l'exception du déplacement des fichiers inutilisés vers `model/coreml_notuse/`.
