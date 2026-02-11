# ~~Plan : Sidecar CoreML persistent (daemon)~~ DONE

> **Statut : IMPLEMENTE** — Le sidecar persistent est en production dans `coreml.rs` + `main.swift`. Ce document est conserve comme reference de design.

## Probleme (resolu)

Chaque appel a `run_inference()` dans `coreml.rs` spawne un nouveau processus `parakeet-coreml` via `Command::new()`. Le sidecar Swift charge les modeles CoreML (`AsrModels.downloadAndLoad`), fait l'inference, retourne le JSON, et meurt. Le cycle complet par transcription :

1. Rust ecrit un WAV temporaire sur disque
2. Rust spawne le binaire `parakeet-coreml`
3. Swift charge les modeles CoreML en memoire (plusieurs secondes)
4. Swift fait l'inference
5. Swift ecrit le JSON sur stdout et le processus meurt
6. Rust parse le resultat et supprime le WAV

Le chargement des modeles (etape 3) se repete a chaque appel. C'est le principal goulot d'etranglement, surtout pour la dictee en temps reel ou les fichiers longs decoupes en chunks.

## Solution : sidecar persistent avec protocole stdin/stdout JSON-lines

Transformer le sidecar en processus longue duree qui :
- Demarre une seule fois (au `load_model()`)
- Charge les modeles une seule fois au demarrage
- Recoit des requetes sur **stdin** (JSON-lines)
- Repond sur **stdout** (JSON-lines)
- Reste vivant jusqu'a arret explicite ou fermeture de l'app

### Pourquoi stdin/stdout plutot que socket/IPC

- Pas de port a gerer ni de conflit
- Gestion native du cycle de vie : quand le parent Tauri meurt, le pipe se ferme et le sidecar se termine proprement
- `std::process::Command` + `Child` gere deja stdin/stdout
- Pas de dependance supplementaire cote Swift ni Rust
- Compatible avec le bundling Tauri existant (`externalBin`)

## Fichiers a modifier

### 1. `src-tauri/sidecar/parakeet-coreml/Sources/main.swift`

**Etat actuel :** CLI one-shot (parse args, charge modeles, transcrit, exit).

**Changement :** Boucle principale qui lit stdin ligne par ligne.

```
Demarrage:
  1. Lire --models <path> depuis les args CLI (seul arg necessaire)
  2. Charger AsrModels + initialiser AsrManager
  3. Ecrire sur stdout: {"status": "ready"}
  4. Boucle infinie: lire une ligne JSON sur stdin

Requete (stdin, une ligne JSON):
  {"audio_path": "/tmp/xxx.wav", "language": "french", "beam_width": 1, "temperature": 0.7, "blank_penalty": 6.0}

Reponse (stdout, une ligne JSON):
  {"text": "bonjour", "confidence": 0.95, "processing_time_ms": 234}

Erreur:
  {"error": "description du probleme"}

Arret:
  - stdin ferme (EOF) → le sidecar quitte proprement
  - Ou requete speciale: {"command": "quit"}
```

Le sidecar garde `AsrManager` en memoire entre les requetes. Seule l'inference est executee a chaque requete.

### 2. `src-tauri/src/engine/coreml.rs`

**Etat actuel :**
- `CoreMLEngine` stocke `model_dir` et `sidecar_path`
- `call_sidecar()` spawne un process a chaque appel via `Command::new().output()`

**Changement :**

```rust
pub struct CoreMLEngine {
    model_dir: Option<PathBuf>,
    sidecar_path: Option<PathBuf>,
    // NOUVEAU: processus persistent
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
}
```

**`load_model()`** : en plus de verifier les chemins, spawne le sidecar avec `Command::new().stdin(Stdio::piped()).stdout(Stdio::piped())`, attend le message `{"status": "ready"}` sur stdout, et stocke les handles.

**`call_sidecar()`** : au lieu de spawner un process, ecrit la requete JSON sur stdin et lit la reponse JSON sur stdout (une ligne).

**`Drop`** : ferme stdin pour signaler au sidecar de quitter. Optionnellement envoie `{"command": "quit"}` avant.

**Note Send/Sync :** Les handles `ChildStdin`/`BufReader<ChildStdout>` ne sont pas `Sync`. Comme `CoreMLEngine` est deja dans un `Mutex<DynamicEngine>` (voir `lib.rs`), c'est compatible — le mutex garantit l'acces exclusif. Les `unsafe impl Send/Sync` existants restent valides si l'acces est serialise par le mutex.

### 3. Pas de modification sur les autres fichiers

- `mod.rs` : le trait `ASREngine` ne change pas (memes signatures)
- `lib.rs` : le chargement/switch de backend ne change pas
- `tauri.conf.json` : le bundling `externalBin` ne change pas
- Frontend : aucun changement

## Protocole JSON-lines detaille

### Requete (Rust → Swift via stdin)

```json
{"audio_path": "/tmp/wakascribe_audio_12345.wav", "language": "french", "beam_width": 1, "temperature": 0.7, "blank_penalty": 6.0}
```

Champs :
- `audio_path` (string, obligatoire) : chemin vers le WAV 16kHz mono 16-bit
- `language` (string) : "auto" | "french" | "english"
- `beam_width` (int) : largeur du beam search
- `temperature` (float) : temperature du softmax
- `blank_penalty` (float) : penalite sur le token blank

### Reponse (Swift → Rust via stdout)

Succes :
```json
{"text": "bonjour le monde", "confidence": 0.95, "processing_time_ms": 234}
```

Erreur :
```json
{"error": "Audio file not found"}
```

### Messages de controle

Ready (Swift → Rust, une seule fois au demarrage) :
```json
{"status": "ready"}
```

Quit (Rust → Swift, optionnel) :
```json
{"command": "quit"}
```

### Regles

- Une requete = une ligne JSON terminee par `\n`
- Une reponse = une ligne JSON terminee par `\n`
- Les logs du sidecar vont sur **stderr** (pas stdout) — c'est deja le cas
- Le sidecar ne doit jamais ecrire autre chose que du JSON sur stdout

## Gestion des erreurs et robustesse

### Crash du sidecar

Si le sidecar crashe en cours de fonctionnement :
1. `stdout_reader.read_line()` retourne `Ok(0)` (EOF) ou `Err`
2. `call_sidecar()` detecte l'erreur
3. Tente un **respawn automatique** : relance le sidecar, attend `{"status": "ready"}`, puis rejoue la requete
4. Si le respawn echoue aussi, remonte l'erreur au caller

### Timeout

Ajouter un timeout sur la lecture stdout (ex: 60 secondes). Si le sidecar ne repond pas :
1. Kill le process enfant
2. Tenter un respawn
3. Remonter l'erreur si le respawn echoue

Implementation possible : `BufReader` avec un thread de lecture + `mpsc::channel` avec `recv_timeout`, ou bien un timeout au niveau plus haut.

### Fermeture propre

- `Drop` sur `CoreMLEngine` : ferme stdin, puis `child.wait()` avec timeout
- Si le sidecar ne quitte pas apres le timeout, `child.kill()`

## Ordre d'implementation

- [x] **Swift** : modifier `main.swift` pour le mode daemon (boucle stdin)
- [x] **Rust** : modifier `CoreMLEngine` pour gerer le processus persistent
- [x] **Test** : verifier que le sidecar se lance, repond, et se ferme proprement
- [x] **Robustesse** : respawn automatique (`call_sidecar_with_respawn`), fermeture propre via `Drop`
- [ ] **Timeout** : non implemente (lecture stdout bloquante sans timeout)

## Gains attendus

- **Premiere transcription** : identique (chargement modeles inclus)
- **Transcriptions suivantes** : suppression du temps de chargement (~2-5s par appel selon la machine)
- **Fichiers longs (chunks)** : gain proportionnel au nombre de chunks (ex: 10 chunks = ~20-50s economisees)
- **Dictee temps reel** : latence reduite drastiquement entre les segments
