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

## 🔐 4. `secure-vault` (App Funcional do Mundo Real: Gerenciador de Senhas)

Exemplo de aplicação **real e totalmente funcional**, além de um simples contador. Demonstra um cofre seguro com criptografia, cálculo de entropia/força de credenciais e persistência local:

- **Plataformas**: Android (Jetpack Compose), iOS (SwiftUI) e Desktop (Rune UI).
- **Core Rust**: 
  - Banco de dados SQLite relacional via `rusqlite` com esquema de tabela `vault_items` e índices de performance.
  - CRUD completo com busca textual em tempo real e filtros por categoria (`Login`, `Note`, `ApiKey`, `CreditCard`).
  - Gerador criptográfico de senhas e avaliador de força/entropia (`evaluate_password`).
  - Métricas agregadas de integridade do cofre (`VaultStats`).
- **Camada FFI**: UniFFI com testes de integração concorrente (`lifecycle_integration_tests.rs`).
- **Desktop (Rune)**: Painel completo com barra lateral de métricas, gerador reativo de senhas e lista de credenciais mascaradas.
- **Android**: Compose Material 3 com filtro por chips, máscara de senhas e gerador reativo.
- **iOS**: SwiftUI com `@Observable`, barra de busca integrada e cartões nativos.
- **CI/CD**: Pipeline do GitHub Actions pré-configurado (`.github/workflows/ci.yml`).

### Como foi gerado:
```bash
jirau new "Secure Vault" \
  --package "io.github.jirau.vault" \
  --android --ios --desktop \
  --sqlite \
  --template vault \
  --theme material3-extended \
  --plugin logger \
  --ci \
  --non-interactive
```

---

## 🚀 Como Executar Qualquer Exemplo

Entre na pasta do projeto desejado e use os comandos do `jirau`:

```bash
cd examples/secure-vault

# Valida o ambiente para todas as plataformas ativas
jirau doctor

# Executa testes unificados (Core Rust, Integração FFI, Android JUnit, iOS XCTest, Desktop)
jirau test all

# Roda o app na plataforma desejada (com seleção inteligente de dispositivo)
jirau run desktop   # Desktop Rune UI
jirau run android   # Android (detecta dispositivo único ou abre menu interativo)
jirau run ios       # iOS Simulator (macOS Apple Silicon)
```
