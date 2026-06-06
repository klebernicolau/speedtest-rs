# 🦀 Speedtest Auditor

> Ferramenta profissional de auditoria de qualidade de enlace de rede, desenvolvida em Rust.

**Autor:** Kleber Nicolau (mr_1r3z3)  
**Versão:** 2.0.0  
**Linguagem:** Rust  
**Plataforma:** Linux / Termux (Android) / qualquer sistema com suporte a Rust  

---

## 📋 Sobre

O **Speedtest Auditor** nasceu da visão de um desenvolvedor de sistemas com background em infraestrutura de redes. Não é um speedtest comum — é uma ferramenta de **auditoria técnica** que mede com precisão a qualidade real do seu enlace, registra histórico e exporta dados estruturados para análise.

---

## ✨ Funcionalidades

- 🔍 Seleção automática do servidor de menor latência (5 candidatos em paralelo)
- 📊 Ping estatístico com 10 amostras — média, mínimo, máximo e jitter (RFC 3550)
- 📥 Teste de download com múltiplas threads paralelas
- 📤 Teste de upload com payload xorshift64 — impede compressão HTTP falsa
- 🖥️ Modo CLI com barra de progresso
- 🎨 Modo TUI com interface visual interativa em tempo real
- 💾 Exportação de resultados em **JSON** ou **CSV** para auditoria histórica

---

## 🚀 Instalação

### Pré-requisitos

- [Rust](https://rustup.rs/) instalado

### Compilar

```bash
git clone https://github.com/mr_1r3z3/speedtest_rs
cd speedtest_rs
cargo build --release
```

### Termux (Android)

```bash
pkg install rust
cargo build --release
```

O binário estará em `target/release/speedtest_rs`.

---

## 🛠️ Uso

```
speedtest_rs [OPÇÕES]
```

| Opção | Abreviação | Padrão | Descrição |
|---|---|---|---|
| `--duration` | `-d` | `10` | Duração dos testes em segundos |
| `--tui` | — | off | Interface visual interativa |
| `--output` | `-o` | — | Salvar resultado em `.json` ou `.csv` |

### Exemplos

```bash
# Teste rápido
./speedtest_rs

# Modo TUI
./speedtest_rs --tui

# Teste de 20s com saída em JSON
./speedtest_rs -d 20 -o auditoria.json

# Teste de campo com saída em CSV
./speedtest_rs -d 15 -o relatorio.csv
```

---

## 📈 Exemplo de saída CLI

```
 Buscando servidores disponíveis...
 Testando latência em 5 servidores...
   42.1ms — Claro (São Paulo)
   310.5ms — SenGi Internet (Manaus)
   89.3ms — Vivo (Campinas)
 Servidor selecionado: Claro (São Paulo)

 Medindo ping (10 amostras)...

==================================================
 Servidor : Claro (São Paulo)
--------------------------------------------------
 Ping Avg : 42.1 ms
 Ping Min : 38.7 ms
 Ping Max : 49.2 ms
 Jitter   : 3.4 ms
==================================================

==================================================
 📊 RESULTADOS
 DL: 35.20 Mbps | UL: 18.50 Mbps
==================================================
```

---

## 💾 Formato de saída

### JSON
```json
[
  {
    "timestamp": "epoch+20229d 23:15:42Z",
    "server_sponsor": "Claro",
    "server_name": "São Paulo",
    "ping_avg_ms": 42.10,
    "ping_min_ms": 38.70,
    "ping_max_ms": 49.20,
    "jitter_ms": 3.40,
    "dl_mbps": 35.20,
    "ul_mbps": 18.50
  }
]
```

### CSV
```
timestamp,server_sponsor,server_name,ping_avg_ms,ping_min_ms,ping_max_ms,jitter_ms,dl_mbps,ul_mbps
epoch+20229d 23:15:42Z,Claro,São Paulo,42.10,38.70,49.20,3.40,35.20,18.50
```

---

## 🧱 Dependências

| Crate | Finalidade |
|---|---|
| `clap` | Parsing de argumentos CLI |
| `reqwest` | Cliente HTTP |
| `anyhow` | Tratamento de erros |
| `serde_json` | Parsing da API de servidores |
| `indicatif` | Barra de progresso |
| `ratatui` | Interface TUI |
| `crossterm` | Backend de terminal |
| `num_cpus` | Paralelismo adaptativo |

---

## 📐 Interpretação dos resultados

| Métrica | Excelente | Bom | Ruim |
|---|---|---|---|
| Ping Avg | < 20ms | 20–80ms | > 80ms |
| Jitter | < 5ms | 5–20ms | > 20ms |
| Download | > 100 Mbps | 10–100 Mbps | < 10 Mbps |
| Upload | > 50 Mbps | 5–50 Mbps | < 5 Mbps |

> Referência VoIP/videoconferência: ITU-T G.114 — ping < 150ms, jitter < 30ms.

---

## 📄 Licença

MIT License — sinta-se livre para usar, modificar e distribuir.

---

*Desenvolvido com 🦀 Rust por Kleber Nicolau (mr_1r3z3)*
