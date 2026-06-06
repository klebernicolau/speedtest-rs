# Speedtest Auditor — Manual de Operação

**Versão:** 2.0.0  
**Linguagem:** Rust  
**Autor:** Kleber Nicolau (mr_1r3z3)  
**Plataforma:** Linux / Termux (Android) / qualquer sistema com suporte a Rust

---

## 1. Descrição

O **Speedtest Auditor** é uma ferramenta de linha de comando desenvolvida em Rust para medição profissional de qualidade de enlace de rede. Diferente de ferramentas comuns de speedtest, este programa foi projetado com foco em **auditoria técnica**, fornecendo métricas precisas de latência, jitter, throughput de download e upload, com suporte a exportação de resultados para histórico de auditoria.

---

## 2. Funcionalidades

- Seleção automática do servidor de menor latência entre os 5 candidatos disponíveis
- Medição de ping com 10 amostras estatísticas (média, mínimo, máximo e jitter)
- Teste de download e upload com múltiplas threads paralelas
- Upload com payload pseudo-aleatório (xorshift64) — impede compressão HTTP falsa
- Dois modos de interface: CLI (texto) e TUI (interface visual interativa)
- Exportação de resultados em JSON ou CSV para auditoria histórica

---

## 3. Compilação

### Pré-requisitos

- Rust toolchain instalado (`rustup`)
- Conexão com a internet (dependências via `cargo`)

### Compilar para produção

```bash
cargo build --release
```

O binário será gerado em:

```
target/release/speedtest_rs
```

### Compilar no Termux (Android)

```bash
pkg install rust
cargo build --release
```

---

## 4. Uso

### Sintaxe

```
speedtest_rs [OPÇÕES]
```

### Opções disponíveis

| Opção | Abreviação | Padrão | Descrição |
|---|---|---|---|
| `--duration` | `-d` | `10` | Duração dos testes de DL e UL em segundos |
| `--tui` | — | desativado | Ativa a interface visual interativa (TUI) |
| `--output` | `-o` | nenhum | Salva o resultado em arquivo JSON ou CSV |
| `--help` | `-h` | — | Exibe a ajuda |
| `--version` | `-V` | — | Exibe a versão |

---

## 5. Exemplos de uso

### Teste simples no modo CLI

```bash
./speedtest_rs
```

### Teste com duração de 20 segundos

```bash
./speedtest_rs -d 20
```

### Teste no modo TUI (interface visual)

```bash
./speedtest_rs --tui
```

### Salvar resultado em JSON

```bash
./speedtest_rs -o auditoria.json
```

### Salvar resultado em CSV

```bash
./speedtest_rs -o relatorio.csv
```

### Teste completo: 15s de duração com saída em JSON

```bash
./speedtest_rs -d 15 -o resultado.json
```

---

## 6. Saída no modo CLI

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

 Resultado salvo em: auditoria.json
```

---

## 7. Saída no modo TUI

Interface visual interativa com barras de progresso em tempo real:

- **Painel superior:** servidor selecionado, ping médio e jitter
- **Barra azul (cyan):** progresso e velocidade de download em Mbps
- **Barra verde:** progresso e velocidade de upload em Mbps
- **Rodapé:** status da execução (`EXECUTANDO...` ou `FINALIZADO`)

**Para encerrar o modo TUI:** `Ctrl+C`

> **Nota:** O modo TUI não gera arquivo de saída. Use o modo CLI com `-o` para auditoria registrada.

---

## 8. Formato dos arquivos de saída

### JSON

Cada execução com `-o arquivo.json` acrescenta um novo registro ao array existente. Se o arquivo não existir, é criado automaticamente.

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

Na primeira execução, o cabeçalho é gerado automaticamente. Execuções subsequentes acrescentam linhas ao final do arquivo.

```
timestamp,server_sponsor,server_name,ping_avg_ms,ping_min_ms,ping_max_ms,jitter_ms,dl_mbps,ul_mbps
epoch+20229d 23:15:42Z,Claro,São Paulo,42.10,38.70,49.20,3.40,35.20,18.50
epoch+20229d 23:45:10Z,Claro,São Paulo,41.80,37.90,48.60,2.90,34.80,17.90
```

> **Observação sobre timestamp:** O campo `timestamp` usa o formato `epoch+Nd HH:MM:SSZ`, calculado a partir do Unix Epoch (1970-01-01) sem dependência de bibliotecas externas como `chrono`.

---

## 9. Arquitetura técnica

### Seleção de servidor

A função `get_best_server` busca os 5 primeiros servidores da API Speedtest e dispara uma thread por servidor em paralelo. Cada thread realiza 1 requisição de warmup seguida de 3 amostras de latência com intervalo de 50ms. O servidor com menor média é selecionado.

### Medição de ping

A função `measure_ping_full` realiza 1 warmup descartado para aquecer a conexão TCP, seguido de 10 amostras com intervalo de 100ms. Calcula:

- **Avg:** média aritmética das amostras
- **Min / Max:** valores extremos observados
- **Jitter:** média das diferenças absolutas entre amostras consecutivas (método RFC 3550)

### Teste de throughput

O teste de download e upload utiliza múltiplas threads paralelas (limitadas a `num_cpus` ou 8, o que for menor). O upload usa payload gerado por **xorshift64** — algoritmo PRNG de alta entropia sem dependências externas — garantindo que a compressão HTTP do servidor não infle artificialmente os resultados.

### Concorrência

A comunicação entre threads utiliza `Arc<AtomicU64>` e `Arc<AtomicBool>` — primitivas de sincronização sem locks, seguras e eficientes para o modelo de medição contínua adotado.

---

## 10. Dependências (Cargo.toml)

| Crate | Finalidade |
|---|---|
| `clap` | Parsing de argumentos CLI |
| `reqwest` (blocking) | Cliente HTTP para testes de rede |
| `anyhow` | Propagação e contexto de erros |
| `serde_json` | Parsing da API de servidores |
| `indicatif` | Barra de progresso no modo CLI |
| `ratatui` | Interface TUI |
| `crossterm` | Backend de terminal para TUI |
| `num_cpus` | Detecção de núcleos para paralelismo |

---

## 11. Interpretação dos resultados

| Métrica | Excelente | Bom | Ruim |
|---|---|---|---|
| Ping Avg | < 20ms | 20–80ms | > 80ms |
| Jitter | < 5ms | 5–20ms | > 20ms |
| Download | > 100 Mbps | 10–100 Mbps | < 10 Mbps |
| Upload | > 50 Mbps | 5–50 Mbps | < 5 Mbps |

> **Referência para VoIP e videoconferência:** ping abaixo de 150ms e jitter abaixo de 30ms são os limites recomendados pelo ITU-T G.114.

---

## 12. Casos de uso em auditoria

### Monitoramento periódico de SLA

```bash
# Executar a cada hora via cron
0 * * * * /home/user/speedtest_rs -d 10 -o /var/log/sla_historico.csv
```

### Comparação antes e depois de manutenção

```bash
./speedtest_rs -d 20 -o antes_manutencao.json
# ... manutenção realizada ...
./speedtest_rs -d 20 -o depois_manutencao.json
```

### Auditoria de enlace 4G/LTE em campo

```bash
./speedtest_rs -d 15 -o auditoria_campo_$(date +%Y%m%d).csv
```

---

*Speedtest Auditor v2.0.0 — Desenvolvido em Rust*
