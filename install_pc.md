# WakaScribe - Installation Windows

## Prérequis pour builder

1. **Node.js** (v18+) : https://nodejs.org/
2. **Rust** : https://rustup.rs/
3. **Visual Studio Build Tools** avec le workload "Desktop development with C++"
4. **WebView2** (inclus dans Windows 10/11 récent) : https://developer.microsoft.com/en-us/microsoft-edge/webview2/

## Prérequis runtime

**OpenVINO Runtime** (pour l'inférence sur CPU Intel) :

1. Télécharger depuis https://github.com/openvinotoolkit/openvino/releases
2. Installer ou extraire l'archive
3. Les DLLs nécessaires :
   - `openvino_c.dll`
   - `openvino.dll`
   - `openvino_ir_frontend.dll` (lecture des modèles .xml/.bin)
   - `openvino_intel_cpu_plugin.dll`
   - `tbb12.dll` (dépendance Intel TBB)

## Build

```powershell
git clone <repo-url> && cd wakascribe
npm install
npm run tauri:build
```

L'installateur est généré dans :
```
src-tauri\target\release\bundle\msi\WakaScribe_0.1.0_x64.msi
```

## Installation des modèles

```powershell
mkdir "%LOCALAPPDATA%\com.wakascribe.app\models\openvino"
xcopy /E model\openvino "%LOCALAPPDATA%\com.wakascribe.app\models\openvino\"
```

Structure attendue :
```
%LOCALAPPDATA%\com.wakascribe.app\
├── wakascribe.db                          (créé automatiquement)
└── models\
    └── openvino\
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

> Le chemin complet est typiquement `C:\Users\<nom>\AppData\Local\com.wakascribe.app\models\openvino\`

## Installation des DLLs OpenVINO

Deux options :

### Option A : Copier à côté de l'exécutable (recommandé)

```powershell
copy "C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release\openvino_c.dll" "C:\Program Files\WakaScribe\"
copy "C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release\openvino.dll" "C:\Program Files\WakaScribe\"
copy "C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release\openvino_ir_frontend.dll" "C:\Program Files\WakaScribe\"
copy "C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release\openvino_intel_cpu_plugin.dll" "C:\Program Files\WakaScribe\"
copy "C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release\tbb12.dll" "C:\Program Files\WakaScribe\"
```

### Option B : Ajouter au PATH système

Ajouter le dossier OpenVINO bin au PATH dans les variables d'environnement Windows :
```
C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release
```

## Vérification

1. Lancer WakaScribe
2. Les logs doivent afficher :
   ```
   Found OpenVINO library at ...
   Found models in app data directory: "C:\\Users\\<nom>\\AppData\\Local\\com.wakascribe.app\\models"
   ```
3. Le moteur OpenVINO doit apparaître comme actif dans les paramètres

## Comparaison Mac / Windows

| | macOS | Windows |
|---|---|---|
| Installateur | `.dmg` | `.msi` |
| Modèles | `~/Library/Application Support/com.wakascribe.app/models/` | `%LOCALAPPDATA%\com.wakascribe.app\models\` |
| OpenVINO | `brew install openvino` | Installer Intel ou copier les DLLs |
| Backends disponibles | OpenVINO (Intel), CoreML (Apple Silicon) | OpenVINO |
