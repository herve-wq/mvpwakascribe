# WakaScribe - Installation macOS

## Prérequis pour builder

1. **Node.js** (v18+) : https://nodejs.org/
2. **Rust** : https://rustup.rs/
3. **Xcode Command Line Tools** : `xcode-select --install`

## Prérequis runtime

### Mac Intel (x86_64)

**OpenVINO Runtime** :
```bash
brew install openvino
```

### Mac Apple Silicon (M1/M2/M3/M4)

Aucune dépendance externe. Le backend CoreML utilise un sidecar Swift (FluidAudio) inclus dans le build.

## Build

```bash
git clone <repo-url> && cd wakascribe
npm install
npm run tauri:build
```

Les artefacts sont générés dans :
```
src-tauri/target/release/bundle/macos/WakaScribe.app
src-tauri/target/release/bundle/dmg/WakaScribe_0.1.0_x64.dmg
```

## Installation des modèles

### Mac Apple Silicon (CoreML)

```bash
mkdir -p ~/Library/Application\ Support/com.wakascribe.app/models/
cp -r model/coreml ~/Library/Application\ Support/com.wakascribe.app/models/
```

Structure attendue :
```
~/Library/Application Support/com.wakascribe.app/
├── wakascribe.db                          (créé automatiquement)
└── models/
    └── coreml/
        ├── Encoder.mlmodelc/
        ├── Decoder.mlmodelc/
        ├── Preprocessor.mlmodelc/
        ├── MelEncoder.mlmodelc/
        └── parakeet_v3_vocab.json
```

### Mac Intel (OpenVINO)

```bash
mkdir -p ~/Library/Application\ Support/com.wakascribe.app/models/
cp -r model/openvino ~/Library/Application\ Support/com.wakascribe.app/models/
```

Structure attendue :
```
~/Library/Application Support/com.wakascribe.app/
├── wakascribe.db                          (créé automatiquement)
└── models/
    └── openvino/
        ├── parakeet_encoder.xml
        ├── parakeet_encoder.bin
        ├── parakeet_decoder.xml
        ├── parakeet_decoder.bin
        ├── parakeet_joint.xml
        ├── parakeet_joint.bin
        ├── parakeet_melspectogram.xml
        ├── parakeet_melspectogram.bin
        └── parakeet_v3_vocab.json
```

## Développement

En mode dev, les modèles sont lus directement depuis `model/` à la racine du projet :

```bash
npm run tauri:dev
```

Aucune copie dans Application Support n'est nécessaire pour le dev.

## Vérification

1. Ouvrir le `.dmg` et glisser WakaScribe dans Applications (ou lancer le `.app` directement)
2. Les logs au démarrage doivent afficher :
   ```
   Found models in app data directory: "/Users/<nom>/Library/Application Support/com.wakascribe.app/models"
   ```
3. Le backend correspondant doit apparaître comme actif dans les paramètres :
   - **Apple Silicon** : CoreML
   - **Intel** : OpenVINO

## Comparaison Intel / Apple Silicon

| | Mac Intel | Mac Apple Silicon |
|---|---|---|
| Backend | OpenVINO | CoreML |
| Dépendance | `brew install openvino` | aucune |
| Modèles | `models/openvino/` | `models/coreml/` |
| Performance | bonne | optimale (Neural Engine) |
