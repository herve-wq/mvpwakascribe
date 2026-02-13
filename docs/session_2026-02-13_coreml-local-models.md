# Session 13 février 2026 — Chargement local des modèles CoreML (FluidAudio)

## Objectif

Permettre au sidecar CoreML de charger les modèles FluidAudio depuis un répertoire local bundlé, sans téléchargement internet au premier lancement.

## Problème résolu

Jusqu'ici, FluidAudio téléchargeait ses modèles (~460 Mo) depuis HuggingFace au premier lancement via `AsrModels.downloadAndLoad(version: .v3)`. Cela imposait une connexion internet obligatoire pour les utilisateurs Apple Silicon.

## Analyse de l'API FluidAudio

FluidAudio (v0.12.0) expose trois méthodes de chargement :

| Méthode | Comportement |
|---|---|
| `AsrModels.downloadAndLoad()` | Télécharge si absent, puis charge (comportement précédent) |
| `AsrModels.load(from: repoDir)` | Charge depuis un répertoire local. Si fichiers manquants, tombe dans le téléchargement |
| `AsrModels.loadFromCache()` | Charge depuis le cache par défaut `~/Library/Application Support/FluidAudio/Models/` |

La méthode `AsrModels.modelsExist(at:, version:)` permet de vérifier la présence des fichiers avant de choisir la stratégie.

### Structure de répertoire requise par FluidAudio

Le répertoire doit s'appeler `parakeet-tdt-0.6b-v3-coreml/` et contenir :
- `Preprocessor.mlmodelc/`
- `Encoder.mlmodelc/`
- `Decoder.mlmodelc/`
- `JointDecision.mlmodelc/` (pas RNNTJoint)
- `parakeet_vocab.json` (pas parakeet_v3_vocab)

### Comportement interne de `load(from:)`

`load(from: directory)` extrait le `parentDirectory` (supprime le dernier composant du chemin), puis ré-appende `repo.folderName` via `DownloadUtils`. Si les fichiers sont présents → chargement local, zéro réseau.

## Implémentation

### 1. `main.swift` — Argument `--models` et chargement local

Ajout de `parseModelsPath()` pour lire `--models <path>` depuis les arguments CLI.

Logique de chargement :
1. Si `--models` fourni → construit `<path>/parakeet-tdt-0.6b-v3-coreml/`
2. Si les modèles existent localement → `AsrModels.load(from:)` sans téléchargement
3. Sinon → fallback `AsrModels.downloadAndLoad()` (télécharge dans le cache FluidAudio par défaut)

### 2. `coreml.rs` — Passage du `model_dir` au sidecar

`spawn_sidecar()` passe maintenant `--models <model_dir>` au processus sidecar. Le `model_dir` est déjà stocké dans `CoreMLEngine` (défini lors de `load_model()`).

### 3. Modèles FluidAudio copiés dans le projet

Copié les 5 fichiers depuis le cache FluidAudio (`~/Library/Application Support/FluidAudio/Models/`) vers `model/coreml/parakeet-tdt-0.6b-v3-coreml/` (461 Mo).

### 4. Nettoyage `model/coreml/`

Les anciens fichiers modèles (custom decoder, HuggingFace git clone) ont été déplacés dans `model/coreml_notuse/` :
- `Decoder.mlmodelc`, `Encoder.mlmodelc`, `Preprocessor.mlmodelc`, `RNNTJoint.mlmodelc`
- `Melspectrogram_15s.mlmodelc`, `ParakeetEncoder_15s.mlmodelc`
- `parakeet_v3_vocab.json`, `README.md`, `.git`, `.gitattributes`

## État final des répertoires modèles

### `model/coreml/` (utilisé)
```
parakeet-tdt-0.6b-v3-coreml/
├── Preprocessor.mlmodelc/   (520 Ko)
├── Encoder.mlmodelc/        (425 Mo)
├── Decoder.mlmodelc/        (23 Mo)
├── JointDecision.mlmodelc/  (12 Mo)
└── parakeet_vocab.json      (148 Ko)
```

### `model/openvino/` (utilisé, inchangé)
```
parakeet_encoder.xml/bin, parakeet_decoder.xml/bin,
parakeet_joint.xml/bin, parakeet_melspectogram.xml/bin,
parakeet_v3_vocab.json — total ~1.1 Go
```

### `model/coreml_notuse/` (archive, non utilisé)
Tous les anciens fichiers CoreML (custom decoder + HuggingFace repo clone).

## Comportement par plateforme (mis à jour)

| Plateforme | Backend | Source des modèles | Offline au 1er lancement ? |
|---|---|---|---|
| macOS ARM (Apple Silicon) | CoreML | Bundlé localement via FluidAudio `load(from:)` | **Oui** |
| macOS Intel | OpenVINO | Bundlé dans l'app | Oui |
| Windows | OpenVINO | Bundlé dans l'app | Oui |

## Fichiers modifiés

| Fichier | Modification |
|---|---|
| `src-tauri/sidecar/parakeet-coreml/Sources/main.swift` | Parsing `--models`, chargement local via `AsrModels.load(from:)` |
| `src-tauri/src/engine/coreml.rs` | `spawn_sidecar()` passe `--models <model_dir>` |
| `model/coreml/parakeet-tdt-0.6b-v3-coreml/` | **Nouveau** — modèles FluidAudio (461 Mo) |
| `model/coreml/*.mlmodelc` | Déplacés vers `model/coreml_notuse/` |

## Validation

- `cargo check` : OK (4 warnings préexistants, 0 erreur)
- `swift build` : OK (build en 6.69s)
