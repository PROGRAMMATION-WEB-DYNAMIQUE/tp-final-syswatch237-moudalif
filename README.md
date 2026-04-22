# SysWatch (Rust)

Projet Rust de supervision reseau avec:
- `agent`: serveur TCP qui expose CPU/RAM/processus de la machine locale
- `controller`: client CLI qui controle une ou plusieurs machines agentes

## Prerequis

- Rust (stable) installe:
  - `rustc --version`
  - `cargo --version`

## Dependances Cargo

Elles sont deja configurees dans `Cargo.toml`:
- `sysinfo = "0.30"` pour les metriques systeme
- `chrono = "0.4"` pour horodatage des logs

Installation des dependances:
```bash
cargo build
```

## Lancer le projet

### Agent (sur les machines controlees)
```bash
cargo run --bin agent
```

Par defaut:
- ecoute sur `0.0.0.0:7878`
- log dans `syswatch.log`

Tu peux changer adresse et fichier log:
```bash
cargo run --bin agent -- 0.0.0.0:7878 custom.log
```

### Controller (sur la machine maitre)
```bash
cargo run --bin controller -- 192.168.1.10:7878 192.168.1.11:7878
```

Si tu ne passes pas d'adresses, il utilise:
- `127.0.0.1:7878`
- `127.0.0.1:7879`

Commandes du controller:
- `list`
- `send <id> <cpu|mem|ps|all|help>`
- `broadcast <cpu|mem|ps|all|help>`
- `quit`

## Ouverture reseau

Sur chaque agent:
- autoriser le port TCP `7878` dans le firewall
- verifier que les machines sont joignables (ping / meme reseau)

