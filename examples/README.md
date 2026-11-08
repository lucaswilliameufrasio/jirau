# 🌟 Exemplos de Projetos Gerados com o Jirau

Esta pasta contém projetos de referência gerados diretamente pela CLI `jirau` para demonstrar diferentes combinações de plataformas, persistência, sistemas de build e kits visuais.

---

## 📱 1. `all-platforms-showcase` (Configuração Completa)

O showcase mais completo do ecossistema, contendo **todas as plataformas suportadas**:

- **Plataformas**: Android (Jetpack Compose), iOS (SwiftUI) e Desktop (Rune UI em Rust puro).
- **Persistência**: SQLite embutido via `rusqlite` integrado ao `StorageEngine` no core Rust.
- **Sistema de Build Android**: JetBrains Amper declarativo (`android/module.yaml`).
- **Tema de UI**: Material 3 Estendido com tokens customizados (`ui/theme/Color.kt`, `Theme.kt`, `Type.kt`).
- **Plugins**: `logger`, `analytics`.
- **Testes**: Suíte de testes de integração FFI end-to-end (`lifecycle_integration_tests.rs`), testes instrumentados Android e testes unitários iOS.

### Como foi gerado:
```bash
jirau new all-platforms-showcase \
  --package com.example.showcase \
  --android --ios --desktop \
  --sqlite \
  --amper \
  --theme material3-extended \
  --plugin logger --plugin analytics \
  --non-interactive
```

---

## 📱 2. `mobile-gradle-starter` (Mobile Tradicional)

Focado em desenvolvimento exclusivamente móvel com o tooling convencional:

- **Plataformas**: Android e iOS.
- **Persistência**: Estado em memória reativo (sem dependência de banco de dados).
- **Sistema de Build Android**: Gradle (Kotlin DSL) padrão com caches agressivos configurados.
- **Tema de UI**: Padrão nativo.

### Como foi gerado:
```bash
jirau new mobile-gradle-starter \
  --package com.example.mobile \
  --android --ios \
  --non-interactive
```

---

## 🖥️ 3. `desktop-first-sqlite` (Desktop de Alta Performance)

Aplicativo Desktop nativo acelerado por GPU:

- **Plataformas**: Desktop (Rune UI).
- **Persistência**: SQLite local via `rusqlite` no core.
- **Sem FFI**: O app desktop consome diretamente a crate em Rust, sem custo de serialização ou bridges intermediárias.

### Como foi gerado:
```bash
jirau new desktop-first-sqlite \
  --package com.example.desktop \
  --desktop \
  --sqlite \
  --non-interactive
```

---

## 🚀 Como Executar Qualquer Exemplo

Entre na pasta do projeto desejado e use os comandos do `jirau`:

```bash
cd examples/all-platforms-showcase

# Valida o ambiente para todas as plataformas ativas
jirau doctor

# Roda testes unitários do core Rust e testes de integração FFI
cargo test --workspace

# Inicia o app desktop com live restart
jirau dev desktop

# Inicia o app Android no dispositivo/emulador conectado
jirau dev android

# Inicia o app iOS no simulador (exclusivo para macOS Apple Silicon)
jirau dev ios
```
