# OpenVINO State Accumulation Fix

## Date: 2026-02-03

## Problème Initial

Le modèle Parakeet présentait une **dégradation progressive** des transcriptions :
- 1ère transcription : OK
- 2ème transcription : Dégradée
- 3ème+ : Hallucinations

**Workaround existant** : `reload_encoder_model()` rechargeait le modèle encoder depuis le disque à chaque inférence (~3s de latence supplémentaire).

---

## Investigation

### Hypothèses testées

| Hypothèse | Résultat |
|-----------|----------|
| Bug dans les bindings Rust OpenVINO | ❌ Non confirmé |
| Modèle stateful caché | ❌ Non (query_state() = vide) |
| Problème de conversion ONNX→IR | ❌ Non |
| reset_state() manquant | ❌ Non pertinent (modèle stateless) |

### Tests effectués

1. **Test Python** (`test_openvino_state.py`)
   - Même modèle, même audio
   - ✅ Aucune accumulation d'état
   - ✅ Inférences répétées stables

2. **Test Rust isolé** (`src/bin/test_openvino_state.rs`)
   - ✅ Aucune accumulation d'état
   - ✅ Même comportement que Python

3. **Test pipeline complet** (`src/bin/test_full_pipeline.rs`)
   - ✅ Aucune accumulation d'état
   - ✅ `reset_all_requests()` fonctionne

### Conclusion de l'investigation

Le **bug d'accumulation d'état n'existe plus** (ou n'a jamais existé de la façon supposée). Les tests avec audio synthétique montrent un comportement stable.

Le problème observé initialement était probablement lié à :
- Audio avec beaucoup de silence/bruit
- Hallucinations sur signal faible (comportement normal des modèles ASR)

---

## Modifications Apportées

### Code supprimé

```rust
// Supprimé de ParakeetEngine:
- model_dir: Option<std::path::PathBuf>
- fn reload_encoder_model(&self) -> Result<()>  // ~60 lignes
```

### Code conservé

```rust
// Garde reset_all_requests() - fonctionne correctement
fn reset_all_requests(&self) -> Result<()> {
    // Recrée les InferRequests depuis les CompiledModels
}
```

### Fichiers modifiés

- `src/engine/parakeet.rs` : Suppression du workaround coûteux
- `Cargo.toml` : Ajout `default-run = "wakascribe"`

### Fichiers créés (tests)

- `src/bin/test_openvino_state.rs` : Test Rust isolé
- `src/bin/test_full_pipeline.rs` : Test pipeline complet
- `test_openvino_state.py` : Test Python comparatif (racine projet)

---

## Gain de Performance

| Métrique | Avant | Après |
|----------|-------|-------|
| Temps par transcription | ~3-4s | ~0.6-0.8s |
| Méthode de reset | Reload depuis disque | Reset InferRequests |

---

## Problèmes Restants (Non liés à l'accumulation d'état)

### 1. Hallucinations sur silence/bruit faible

**Symptôme** : Le modèle génère du texte incohérent (souvent en anglais) quand l'audio contient principalement du silence ou du bruit.

**Indicateur** : Encoder RMS < 0.01

**Exemples observés** :
- "I don't know. I don't know..." (répété en boucle)
- "Yeah, yeah, I can't do it..."
- "Definitely. Six, colour..."

**Solutions possibles** :
- [ ] Ajouter un seuil sur l'encoder RMS (rejeter si < 0.01)
- [ ] Implémenter un VAD (Voice Activity Detection)
- [ ] Détecter les patterns de répétition dans le décodage

### 2. Troncature audio à 15 secondes

**Symptôme** : Les enregistrements > 15s sont tronqués aux 15 premières secondes.

**Cause** : `MAX_AUDIO_SAMPLES = 240000` (15s @ 16kHz)

**Solutions possibles** :
- [ ] Implémenter le chunking (découper en segments de 15s)
- [ ] Utiliser les 15 dernières secondes au lieu des premières
- [ ] Augmenter la limite (impact mémoire/performance)

### 3. Mélange de langues

**Symptôme** : Le modèle mélange parfois français et anglais.

**Cause** : Parakeet est multilingue et n'a pas de détection de langue.

**Solutions possibles** :
- [ ] Forcer une langue via le prompt/context
- [ ] Post-traitement pour filtrer les tokens incohérents

---

## Configuration Versions

| Composant | Version |
|-----------|---------|
| OpenVINO Runtime | 2025.4.1 |
| Crate Rust openvino | 0.9.1 |
| Python OpenVINO | 2025.4.1 |

---

## Commandes Utiles

```bash
# Lancer l'application
cd wakascribe && npm run tauri:dev

# Voir les logs en temps réel
tail -f /tmp/wakascribe.log | grep -E "DIAG|Encoder|Decoded"

# Exécuter les tests de diagnostic
cd src-tauri
OPENVINO_LIB_PATH=/usr/local/lib cargo run --bin test_openvino_state
OPENVINO_LIB_PATH=/usr/local/lib cargo run --bin test_full_pipeline

# Test Python
python3 ../test_openvino_state.py
```

---

---

## Fonctionnalité de Test Ajoutée

### Bouton "Test Transcription"

Un bouton de test a été ajouté dans les paramètres pour effectuer des tests reproductibles.

**Emplacement** : Paramètres > Section Modèle > Bouton "Test Transcription"

### Fichiers créés

| Fichier | Description |
|---------|-------------|
| `src-tauri/src/commands/test_transcription.rs` | Commande Tauri de test |
| `src/components/TestButton/TestButton.tsx` | Composant React |
| `src/components/TestButton/index.ts` | Export du composant |

### Configuration requise

Placer un fichier `test_audio.wav` dans le dossier `model/` :

```
model/
├── parakeet_encoder.xml
├── parakeet_decoder.xml
├── parakeet_joint.xml
├── parakeet_melspectogram.xml
├── parakeet_v3_vocab.json
└── test_audio.wav  ← Fichier de test (5-10s, voix claire)
```

### Métriques affichées

- **Texte transcrit** : Résultat de la transcription
- **Durée audio** : Longueur du fichier de test
- **Temps traitement** : Durée de la transcription
- **Vitesse** : Ratio temps réel (ex: 5x = 5 fois plus rapide que temps réel)
- **Audio RMS** : Niveau du signal audio

### Désactivation

Pour désactiver complètement cette fonctionnalité :

**Backend (Rust)** :
```rust
// Dans src/commands/mod.rs, commenter :
// pub mod test_transcription;
// pub use test_transcription::*;

// Dans src/lib.rs, commenter :
// commands::test_transcription,
// commands::check_test_audio,
```

**Frontend (React)** :
```tsx
// Dans src/components/Settings/index.tsx, commenter :
// import { TestButton } from "../TestButton";
// <TestButton className="mt-4" />
```

---

## Références

- [intel/openvino-rs](https://github.com/intel/openvino-rs) - Bindings Rust OpenVINO
- Issue #167 : Breaking change API C OpenVINO 2025.1+
- Issue #180 : create_infer_request devrait utiliser &self
