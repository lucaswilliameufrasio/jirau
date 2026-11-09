[English](README.md) | Português (Brasil)

# 🌟 Projetos de Exemplo Gerados com o jirau

Projetos de referência gerados diretamente pela CLI `jirau` (template v0.4.0). Eles são regenerados a partir dos templates, então sempre correspondem ao que `jirau new` produz hoje. Cada um tem um snapshot `.jirau/template-base/`, que o `jirau upgrade` usa como base do 3-way merge.

Todos foram validados em simulador iOS (Xcode 27.2) e emulador Android (API 36, x86_64): `jirau run`, `jirau test`, `cargo fmt --check`, `cargo clippy -D warnings` e `cargo test`.

---

## 📱 1. `all-platforms-showcase` (configuração completa)

- **Plataformas**: Android (Jetpack Compose), iOS (SwiftUI) e Desktop (Rune UI em Rust puro).
- **Persistência**: SQLite embutido via `rusqlite` no núcleo Rust (`StorageEngine`).
- **Build Android**: Gradle (Kotlin DSL + version catalog). O Gradle compila o núcleo Rust via `scripts/build-android-ffi.sh` antes de cada build.
- **Tema de UI**: Material 3 estendido. **Plugins**: `logger`, `analytics`.
- **Testes**: integração UniFFI, testes instrumentados Android (ViewModel + UI Compose contra o núcleo Rust real) e XCTest no iOS.

```bash
jirau new all-platforms-showcase \
  --package com.example.showcase \
  --android --ios --desktop \
  --sqlite \
  --theme material3-extended \
  --plugin logger --plugin analytics \
  --non-interactive
```

> Antes era gerado com `--amper`. O suporte ao JetBrains Amper foi removido (o modo plugin do Gradle foi descontinuado e o Amper standalone não tem versão estável), então agora usa Gradle.

---

## 📱 2. `mobile-gradle-starter` (só mobile)

- **Plataformas**: Android e iOS.
- **Estado**: contador em memória que vive no núcleo Rust (`AppEngine` via UniFFI); as UIs nativas só o chamam.

```bash
jirau new mobile-gradle-starter \
  --package com.example.mobile \
  --android --ios \
  --non-interactive
```

---

## 🖥️ 3. `desktop-first-sqlite` (desktop)

- **Plataformas**: Desktop (Rune UI, acelerado por GPU).
- **Persistência**: SQLite local via `rusqlite` no núcleo.
- **Sem FFI**: o app desktop usa a crate Rust diretamente.

```bash
jirau new desktop-first-sqlite \
  --package com.example.desktop \
  --desktop \
  --sqlite \
  --non-interactive
```

---

## 🔐 4. `secure-vault` (app real: gerenciador de senhas)

- **Plataformas**: Android (Jetpack Compose), iOS (SwiftUI) e Desktop (Rune UI).
- **Núcleo Rust**: SQLite (`vault_items`) com CRUD completo, busca, filtros por categoria, gerador de senhas com CSPRNG, avaliador de força e métricas de saúde do cofre.
- **Criptografia**: cada segredo é cifrado com AES-256-GCM-SIV (`aws-lc-rs`) e amarrado ao seu item. A chave de 32 bytes é gerada pelo núcleo Rust e guardada no Keychain do iOS ou embrulhada pelo Android Keystore; nunca vai para o banco. Bancos antigos em texto puro são migrados automaticamente. Títulos, usuários e tags continuam como metadados pesquisáveis.
- **Apps**: UIs nativas completas (criar, editar, revelar, copiar, favoritar, excluir, buscar, gerar senha) com testes contra o núcleo real.
- **Fonte do template**: este exemplo é a fonte da verdade do template `vault`. Depois de editá-lo, rode `python3 scripts/sync-vault-template.py`.

```bash
jirau new "Secure Vault" \
  --package io.github.jirau.vault \
  --android --ios --desktop \
  --sqlite \
  --template vault \
  --theme material3-extended \
  --plugin logger \
  --ci \
  --non-interactive
```

---

## 🚀 Como executar qualquer exemplo

```bash
cd examples/secure-vault

jirau doctor        # valida o ambiente das plataformas ativas
jirau test all      # núcleo Rust, FFI, Android (dispositivo conectado), iOS (simulador), desktop
jirau run android   # compila, instala e abre no dispositivo/emulador conectado
jirau run ios       # compila, instala e abre no simulador
jirau run desktop
```
