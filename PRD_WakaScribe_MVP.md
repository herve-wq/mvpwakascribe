# PRD — WakaScribe MVP
## Mode Speech-to-Text Offline avec NVIDIA Parakeet

**Version** : 1.0  
**Date** : 30 janvier 2026  
**Statut** : Draft  
**Plateforme cible MVP** : MacBook Pro Intel (2019)

---

## 1. Résumé exécutif

### 1.1 Vision produit

WakaScribe MVP est une application de bureau permettant la transcription vocale entièrement offline, optimisée pour les machines Intel via le framework OpenVINO. L'application combine dictée en temps réel et transcription de fichiers audio, avec une interface moderne et des fonctionnalités d'édition intégrées.

### 1.2 Objectifs du MVP

| Objectif | Métrique de succès |
|----------|-------------------|
| Transcription temps réel fluide | Latence < 500ms entre parole et affichage |
| Performance acceptable sur Intel | Facteur vitesse ≥ 4x temps réel |
| Précision française | Taux d'erreur mot (WER) < 15% en conditions normales |
| Stabilité | Aucun crash sur sessions de 30 minutes |

### 1.3 Configuration cible

| Composant | Spécification |
|-----------|---------------|
| Processeur | Intel Core i9 8 cœurs @ 2.4 GHz |
| GPU dédié | AMD Radeon Pro 5500M 4 Go (non utilisé pour MVP) |
| GPU intégré | Intel UHD Graphics 630 1536 Mo (cible OpenVINO) |
| OS | macOS (version à préciser) |

---

## 2. Périmètre fonctionnel

### 2.1 Fonctionnalités "Must Have"

#### 2.1.1 Transcription

| ID | Fonctionnalité | Description |
|----|----------------|-------------|
| F-001 | Dictée temps réel | Capture micro → transcription → affichage instantané |
| F-002 | Transcription fichiers | Import audio (wav, mp3, m4a, ogg) → transcription batch |
| F-003 | Langue française | Support natif du français avec vocabulaire courant |
| F-004 | Mode offline complet | Aucune connexion internet requise après installation |

#### 2.1.2 Interface utilisateur

| ID | Fonctionnalité | Description |
|----|----------------|-------------|
| F-010 | Visualisation waveform | Forme d'onde audio en temps réel pendant l'enregistrement |
| F-011 | Thème sombre/clair | Basculement entre les deux modes |
| F-012 | Panneau paramètres | Sélection micro, préférences, gestion modèle |
| F-013 | Timestamps | Marqueurs temporels dans la transcription |
| F-014 | Indicateur confiance | Score de fiabilité par segment transcrit |

#### 2.1.3 Édition et export

| ID | Fonctionnalité | Description |
|----|----------------|-------------|
| F-020 | Édition inline | Correction du texte transcrit directement dans l'interface |
| F-021 | Export texte | Copier dans le presse-papier |
| F-022 | Export fichier | Sauvegarde en .txt et .docx |
| F-023 | Historique | Liste des transcriptions passées avec recherche |

#### 2.1.4 Raccourcis et ergonomie

| ID | Fonctionnalité | Description |
|----|----------------|-------------|
| F-030 | Raccourcis globaux | Démarrer/stopper dictée depuis n'importe quelle app |
| F-031 | Raccourci pause | Mettre en pause sans arrêter la session |
| F-032 | Raccourci export rapide | Copier la dernière transcription |

### 2.2 Hors périmètre MVP

- Support multi-langue (hors français)
- Identification des locuteurs (diarization)
- Synchronisation cloud
- Version Windows/Linux
- Utilisation du GPU AMD Radeon
- Ponctuation automatique avancée
- Commandes vocales

---

## 3. Architecture technique

### 3.1 Vue d'ensemble

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Tauri                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Frontend React                        │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐  │   │
│  │  │ Recorder │  │ Waveform │  │  Editor  │  │ History │  │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └─────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              │ Tauri Commands (IPC)             │
│                              ▼                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Backend Rust                          │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐  │   │
│  │  │  Audio   │  │ OpenVINO │  │  Export  │  │ Storage │  │   │
│  │  │ Capture  │  │  Engine  │  │  Module  │  │  (SQLite)│  │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └─────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Intel UHD 630 (via OpenVINO)                │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Stack technologique

| Couche | Technologie | Justification |
|--------|-------------|---------------|
| Framework app | Tauri 2.x | Léger, natif, Rust backend |
| Frontend | React 18 + TypeScript | Écosystème riche, compétences disponibles |
| Styling | Tailwind CSS | Rapidité de développement, thèmes faciles |
| Backend | Rust | Performance, sécurité mémoire |
| Moteur STT | Parakeet TDT v3 (OpenVINO) | Optimisé Intel, 4x+ temps réel |
| Base de données | SQLite (via rusqlite) | Léger, embarqué, fiable |
| Audio | cpal (Rust) | Cross-platform, bas niveau |
| Waveform | wavesurfer.js | Mature, performant |

### 3.3 Modèle Parakeet

| Attribut | Valeur |
|----------|--------|
| Nom | parakeet-tdt-0.6b-v3-ov |
| Format | OpenVINO IR (.xml + .bin) |
| Taille | ~600 Mo |
| Source | Hugging Face (FluidInference) |
| Langues | ~25 dont français |
| Architecture | Token-Duration-Transducer (TDT) |

### 3.4 Intégration OpenVINO

```rust
// Pseudo-code d'initialisation
use openvino::{Core, Tensor};

pub struct ParakeetEngine {
    compiled_model: CompiledModel,
    sample_rate: u32,
}

impl ParakeetEngine {
    pub fn new(model_path: &Path) -> Result<Self> {
        let core = Core::new()?;
        let model = core.read_model(model_path)?;
        
        // Utilisation du GPU Intel UHD 630
        let compiled_model = core.compile_model(&model, "GPU")?;
        
        Ok(Self {
            compiled_model,
            sample_rate: 16000,
        })
    }
    
    pub async fn transcribe(&self, audio: &[f32]) -> Result<TranscriptionResult> {
        // Préparation du tensor d'entrée
        // Inférence
        // Décodage des tokens
        // Retour avec timestamps et confiance
    }
}
```

---

## 4. Spécifications détaillées

### 4.1 Écran principal — Mode dictée

```
┌────────────────────────────────────────────────────────────────┐
│  WakaScribe                              [─] [□] [×]  ☀️/🌙   │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                    Waveform Visualizer                    │ │
│  │  ∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿ │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│     ⏺ REC  00:01:23        🎤 MacBook Pro Microphone  ▼       │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ [00:00:05] Bonjour, ceci est un test de dictée vocale.   │ │
│  │            Confiance: ████████░░ 85%                      │ │
│  │                                                           │ │
│  │ [00:00:12] Le système fonctionne correctement et la      │ │
│  │            transcription apparaît en temps réel.          │ │
│  │            Confiance: █████████░ 92%                      │ │
│  │                                                           │ │
│  │ [00:00:20] _                                              │ │
│  │            (en cours de transcription...)                 │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌─────────────────────┐ │
│  │ ⏹ Stop │  │ ⏸ Pause│  │📋 Copier│  │ 💾 Exporter...    ▼│ │
│  └────────┘  └────────┘  └────────┘  └─────────────────────┘ │
│                                                                │
│  💡 Raccourcis: ⌘+Shift+R (enregistrer) • ⌘+Shift+S (stop)   │
└────────────────────────────────────────────────────────────────┘
```

### 4.2 Écran — Mode transcription fichier

```
┌────────────────────────────────────────────────────────────────┐
│  WakaScribe                              [─] [□] [×]  ☀️/🌙   │
├────────────────────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────────┐                                  │
│  │ Dictée  │ │ Fichier ▼  │                                   │
│  └─────────┘ └─────────────┘                                  │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                                                           │ │
│  │     📁 Glissez un fichier audio ici                      │ │
│  │        ou cliquez pour sélectionner                       │ │
│  │                                                           │ │
│  │     Formats supportés: .wav .mp3 .m4a .ogg               │ │
│  │                                                           │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ── Fichier en cours ────────────────────────────────────────  │
│                                                                │
│  📄 interview_client.mp3                                      │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░  67%  |  02:34 / 03:50       │
│  Vitesse: 4.2x temps réel                                     │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ [Transcription en cours d'apparition...]                 │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### 4.3 Panneau des paramètres

```
┌────────────────────────────────────────────────────────────────┐
│  Paramètres                                             [×]    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  🎤 ENTRÉE AUDIO                                              │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Microphone:  [MacBook Pro Microphone           ▼]        │ │
│  │ Niveau:      ████████░░░░░░░░░░░░░░░░  -12 dB            │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  🌐 LANGUE                                                    │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Langue de transcription:  [Français            ▼]        │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  🧠 MODÈLE                                                    │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Moteur:      Parakeet TDT v3 (OpenVINO)                  │ │
│  │ Statut:      ✅ Chargé (GPU Intel UHD 630)               │ │
│  │ Mémoire:     ~1.2 Go utilisés                            │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ⌨️ RACCOURCIS GLOBAUX                                        │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Démarrer/Arrêter dictée:  [⌘ + Shift + R      ]          │ │
│  │ Pause:                    [⌘ + Shift + P      ]          │ │
│  │ Copier transcription:     [⌘ + Shift + C      ]          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  🎨 APPARENCE                                                 │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Thème:       ◉ Clair  ○ Sombre  ○ Système                │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    Enregistrer                          │  │
│  └────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

### 4.4 Historique des transcriptions

```
┌────────────────────────────────────────────────────────────────┐
│  Historique                                             [×]    │
├────────────────────────────────────────────────────────────────┤
│  🔍 [Rechercher dans l'historique...                    ]     │
│                                                                │
│  ── Aujourd'hui ───────────────────────────────────────────── │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 📝 Dictée — 14:32                           Durée: 5:23  │ │
│  │ "Bonjour, ceci est un test de dictée vocale..."          │ │
│  │ [Ouvrir] [Exporter] [Supprimer]                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 📄 interview_client.mp3 — 11:15             Durée: 3:50  │ │
│  │ "Merci d'avoir accepté cette interview..."               │ │
│  │ [Ouvrir] [Exporter] [Supprimer]                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ── Hier ──────────────────────────────────────────────────── │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 📝 Dictée — 09:45                           Durée: 12:07 │ │
│  │ "Notes de réunion projet Alpha..."                       │ │
│  │ [Ouvrir] [Exporter] [Supprimer]                          │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

---

## 5. Flux utilisateur

### 5.1 Dictée temps réel

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Lancer    │────▶│   Charger   │────▶│   Prêt à    │
│    l'app    │     │   modèle    │     │   dicter    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                    ┌──────────────────────────┘
                    ▼
              ┌───────────┐     ┌───────────┐     ┌───────────┐
              │  Clic REC │────▶│  Capture  │────▶│Transcription│
              │ ou ⌘⇧R   │     │   audio   │     │ temps réel │
              └───────────┘     └───────────┘     └──────┬──────┘
                                                        │
                    ┌───────────────────────────────────┘
                    ▼
              ┌───────────┐     ┌───────────┐     ┌───────────┐
              │ Clic STOP │────▶│  Édition  │────▶│  Export   │
              │ ou ⌘⇧S   │     │  optionnel│     │ txt/docx  │
              └───────────┘     └───────────┘     └───────────┘
```

### 5.2 Transcription de fichier

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Onglet     │────▶│ Drag & drop │────▶│ Validation  │
│  "Fichier"  │     │   fichier   │     │   format    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
              ┌────────────────────────────────┘
              ▼
        ┌───────────┐     ┌───────────┐     ┌───────────┐
        │ Démarrage │────▶│ Progression│────▶│Transcription│
        │   auto    │     │   batch    │     │  complète  │
        └───────────┘     └───────────┘     └──────┬──────┘
                                                   │
              ┌────────────────────────────────────┘
              ▼
        ┌───────────┐     ┌───────────┐
        │  Édition  │────▶│  Export   │
        │ optionnel │     │ txt/docx  │
        └───────────┘     └───────────┘
```

---

## 6. Modèle de données

### 6.1 Schéma SQLite

```sql
-- Table des transcriptions
CREATE TABLE transcriptions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    source_type TEXT NOT NULL, -- 'dictation' | 'file'
    source_name TEXT,          -- nom du fichier si applicable
    duration_ms INTEGER,
    language TEXT DEFAULT 'fr',
    raw_text TEXT,
    edited_text TEXT,
    is_edited BOOLEAN DEFAULT FALSE
);

-- Table des segments avec timestamps et confiance
CREATE TABLE segments (
    id TEXT PRIMARY KEY,
    transcription_id TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL,           -- 0.0 à 1.0
    FOREIGN KEY (transcription_id) REFERENCES transcriptions(id)
);

-- Table des préférences utilisateur
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Index pour la recherche
CREATE INDEX idx_transcriptions_created ON transcriptions(created_at);
CREATE INDEX idx_segments_transcription ON segments(transcription_id);
CREATE VIRTUAL TABLE transcriptions_fts USING fts5(raw_text, edited_text);
```

### 6.2 Structure TypeScript (Frontend)

```typescript
interface Transcription {
  id: string;
  createdAt: Date;
  updatedAt: Date;
  sourceType: 'dictation' | 'file';
  sourceName?: string;
  durationMs: number;
  language: string;
  segments: Segment[];
  isEdited: boolean;
}

interface Segment {
  id: string;
  startMs: number;
  endMs: number;
  text: string;
  confidence: number;
}

interface Settings {
  theme: 'light' | 'dark' | 'system';
  language: string;
  inputDevice: string;
  shortcuts: {
    toggleRecording: string;
    pause: string;
    copy: string;
  };
}
```

---

## 7. Raccourcis clavier

| Action | Raccourci | Contexte |
|--------|-----------|----------|
| Démarrer/Arrêter dictée | ⌘ + Shift + R | Global |
| Pause dictée | ⌘ + Shift + P | Global |
| Copier transcription | ⌘ + Shift + C | Global |
| Nouvelle dictée | ⌘ + N | In-app |
| Ouvrir fichier | ⌘ + O | In-app |
| Exporter | ⌘ + E | In-app |
| Paramètres | ⌘ + , | In-app |
| Historique | ⌘ + H | In-app |

---

## 8. Exigences non fonctionnelles

### 8.1 Performance

| Métrique | Cible | Méthode de mesure |
|----------|-------|-------------------|
| Latence dictée | < 500 ms | Temps entre fin de phrase et affichage |
| Vitesse batch | ≥ 4x temps réel | Durée traitement / durée audio |
| Temps démarrage | < 5 s | Splash screen à prêt |
| Chargement modèle | < 10 s | Premier lancement |
| Mémoire RAM | < 2 Go | Pic d'utilisation |

### 8.2 Fiabilité

| Exigence | Détail |
|----------|--------|
| Disponibilité | Fonctionne 100% offline après installation |
| Stabilité | Pas de crash sur sessions de 30 min |
| Récupération | Sauvegarde automatique toutes les 30 secondes |
| Données | Aucune perte de transcription en cas de crash |

### 8.3 Sécurité

| Exigence | Implémentation |
|----------|----------------|
| Données locales | Tout stocké localement (SQLite) |
| Pas de télémétrie | Aucune donnée envoyée à l'extérieur |
| Permissions | Micro uniquement |

---

## 9. Dépendances et risques

### 9.1 Dépendances techniques

| Dépendance | Version | Risque | Mitigation |
|------------|---------|--------|------------|
| OpenVINO Runtime | 2024.x | Compatibilité macOS Intel | Tests sur machine cible |
| Modèle Parakeet | v3-ov | Disponibilité HuggingFace | Backup local |
| Tauri | 2.x | Stabilité macOS | Version LTS |
| openvino-rs | Latest | Maturité crate | Fallback Python sidecar |

### 9.2 Risques identifiés

| Risque | Impact | Probabilité | Mitigation |
|--------|--------|-------------|------------|
| Performance insuffisante sur i9 | Élevé | Moyenne | Quantification INT8, optimisation pipeline |
| Crate openvino-rs instable | Moyen | Faible | Sidecar Python avec PyInstaller |
| Qualité STT français | Moyen | Faible | Fine-tuning ou fallback Whisper |
| Surchauffe CPU | Moyen | Moyenne | Limitation framerate, pauses adaptatives |

---

## 10. Roadmap et jalons

### Phase 1 — Fondations (Semaines 1-2)

| Tâche | Livrable |
|-------|----------|
| Setup projet Tauri + React | Repo initialisé, build fonctionnel |
| Intégration OpenVINO | Engine Rust chargant le modèle |
| Test inférence basique | Transcription d'un fichier WAV |

### Phase 2 — Core Features (Semaines 3-4)

| Tâche | Livrable |
|-------|----------|
| Capture audio temps réel | Stream micro → buffer |
| Pipeline STT streaming | Transcription incrémentale |
| UI dictée basique | Écran principal fonctionnel |

### Phase 3 — Interface complète (Semaines 5-6)

| Tâche | Livrable |
|-------|----------|
| Waveform visualizer | Intégration wavesurfer.js |
| Thème sombre/clair | Système de theming |
| Panneau paramètres | UI complète |
| Mode fichier | Import et transcription batch |

### Phase 4 — Polish (Semaines 7-8)

| Tâche | Livrable |
|-------|----------|
| Historique + SQLite | Persistance complète |
| Export txt/docx | Module d'export |
| Raccourcis globaux | Tauri global shortcuts |
| Édition inline | Correction post-transcription |
| Tests et bugs | MVP stable |

---

## 11. Critères d'acceptation MVP

### 11.1 Fonctionnels

- [ ] L'utilisateur peut dicter en français et voir le texte apparaître en < 500ms
- [ ] L'utilisateur peut importer un fichier audio et obtenir la transcription
- [ ] Chaque segment affiche son timestamp et son score de confiance
- [ ] L'utilisateur peut éditer le texte transcrit
- [ ] L'utilisateur peut exporter en .txt et .docx
- [ ] L'historique conserve toutes les transcriptions
- [ ] Les raccourcis globaux fonctionnent depuis n'importe quelle app
- [ ] Le thème sombre/clair fonctionne
- [ ] Le panneau paramètres permet de changer le micro

### 11.2 Techniques

- [ ] L'application fonctionne 100% offline
- [ ] La vitesse de transcription batch est ≥ 4x temps réel
- [ ] La RAM utilisée reste < 2 Go
- [ ] Aucun crash sur session de 30 minutes
- [ ] Le build produit un .dmg installable

---

## 12. Annexes

### 12.1 Ressources

| Ressource | URL |
|-----------|-----|
| Parakeet OpenVINO | https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v3-ov |
| openvino-rs | https://github.com/intel/openvino-rs |
| Tauri docs | https://tauri.app/v2/guides/ |
| wavesurfer.js | https://wavesurfer-js.org/ |

### 12.2 Glossaire

| Terme | Définition |
|-------|------------|
| TDT | Token-Duration-Transducer, architecture du modèle Parakeet |
| OpenVINO | Toolkit Intel pour l'inférence optimisée |
| WER | Word Error Rate, taux d'erreur par mot |
| RTF | Real-Time Factor, ratio durée traitement / durée audio |
| Sidecar | Exécutable externe piloté par l'app principale |

---

*Document généré le 30 janvier 2026*
