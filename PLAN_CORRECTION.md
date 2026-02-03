# Plan de Correction WakaScribe STT Engine

## Analyse comparative

J'ai comparé le code de WakaScribe (non fonctionnel) avec celui de Scribe (fonctionnel). Voici les différences critiques identifiées.

---

## BUG CRITIQUE #1 : blank_id incorrect dans le vocabulaire

**Fichier:** `src-tauri/src/engine/decoder.rs:47`

**WakaScribe (incorrect):**
```rust
let blank_id = tokens.len(); // ~1031, FAUX!
```

**Scribe (correct):**
```rust
let blank_token_id = 8192; // Valeur correcte pour Parakeet TDT
```

**Impact:** Le modèle TDT utilise le token ID 8192 comme "blank" (pas de sortie). Si on utilise `tokens.len()` (~1031), le décodeur ne reconnaîtra jamais les blanks et produira du charabia.

**Correction:**
```rust
// Dans Vocabulary::load_json()
let blank_id = 8192; // Token blank pour Parakeet TDT (hardcodé)
```

---

## BUG #2 : Structure de données du vocabulaire sous-optimale

**WakaScribe:** Utilise `Vec<String>` indexé par token ID, ce qui pose problème car le vocabulaire a des IDs non-contigus (0-1030 puis 8192 pour blank).

**Scribe:** Utilise `HashMap<u32, String>` qui gère correctement les IDs épars.

**Correction:** Soit :
1. Utiliser `HashMap<u32, String>` comme Scribe
2. Ou garder `Vec` mais ignorer les tokens >= vocab.len() dans `decode_single`

---

## BUG #3 : Filtrage excessif des "special tokens"

**Fichier:** `src-tauri/src/engine/decoder.rs:65-71`

**WakaScribe:**
```rust
pub fn is_special_token(&self, id: usize) -> bool {
    if id >= self.tokens.len() {
        return true; // Traite TOUT token >= len comme spécial
    }
    let token = &self.tokens[id];
    token.starts_with('<') && token.ends_with('>')
}
```

**Problème:** Cela filtre potentiellement des tokens valides.

**Scribe:** N'utilise pas cette logique, ne filtre que le blank (8192).

**Correction:** Simplifier la logique de décodage pour ne filtrer que le blank_id.

---

## BUG #4 : DeviceType pour OpenVINO

**Fichier:** `src-tauri/src/engine/parakeet.rs:148`

**WakaScribe:**
```rust
core.compile_model(&model, "CPU".into())
```

**Scribe:**
```rust
core.compile_model(&model, DeviceType::CPU)
```

**Impact:** Peut fonctionner, mais utiliser l'enum `DeviceType::CPU` est plus sûr et idiomatique.

**Correction:**
```rust
use openvino::DeviceType;
// ...
core.compile_model(&model, DeviceType::CPU)
```

---

## Résumé des fichiers à modifier

| Fichier | Modifications |
|---------|--------------|
| `src-tauri/src/engine/decoder.rs` | Corriger blank_id = 8192, simplifier is_special_token |
| `src-tauri/src/engine/parakeet.rs` | Utiliser DeviceType::CPU |

---

## Plan d'implémentation

### Étape 1 : Corriger decoder.rs

1. Changer `blank_id = tokens.len()` en `blank_id = 8192`
2. Simplifier `is_special_token()` pour ne vérifier que blank_id
3. Mettre à jour `decode_single()` pour gérer correctement les IDs hors limites

### Étape 2 : Corriger parakeet.rs

1. Ajouter `use openvino::DeviceType;`
2. Remplacer `"CPU".into()` par `DeviceType::CPU`

### Étape 3 : Tester

1. Recompiler avec `npm run tauri:dev`
2. Tester avec un fichier audio ou enregistrement
3. Vérifier les logs pour confirmer que le décodage TDT fonctionne

---

## Détails techniques

### Constantes du modèle Parakeet TDT v3

| Paramètre | Valeur |
|-----------|--------|
| Sample rate | 16000 Hz |
| Max audio | 240000 samples (15s) |
| Mel features | 128 |
| Max mel frames | 1501 |
| Encoder output dim | 1024 |
| Decoder hidden dim | 640 |
| Decoder layers | 2 |
| **Vocab size** | **8193** (0-8192) |
| **Blank token ID** | **8192** |
| Duration classes | 5 (1-5 frames) |

### Format du vocabulaire

Le fichier `parakeet_v3_vocab.json` contient :
- Tokens 0-1030 : vocabulaire SentencePiece (avec `▁` pour les espaces)
- Token 0 : `<unk>`
- Token 8192 : blank (implicite, pas dans le fichier JSON)
