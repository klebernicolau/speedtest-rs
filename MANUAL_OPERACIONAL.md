# Manual Operacional - SpeedTest-RS ⚡

Este documento detalha a operação da CLI e a arquitetura interna das funções do sistema.

## 🕹 Guia de Comandos (CLI Help)

O binário aceita os seguintes argumentos para controle de execução:

| Comando | Função | Descrição Técnica |
|:--- |:--- |:--- |
| `--help` | Ajuda | Exibe a lista de comandos e encerra. |
| `--threads <n>` | Concorrência | Define o número de workers paralelos para o teste de carga. |
| `--json` | Parse Manual | Formata a saída em JSON para integração com outras ferramentas. |
| `--timeout <s>` | Network Limit | Define o tempo máximo de espera por socket antes do drop. |
| `--server <url>` | Target | Permite definir um servidor específico para o teste. |

---

## 🏗 Documentação de Funções Internas (Technical Reference)

Abaixo estão explicadas as funções core do programa, focadas em performance e segurança de memória.

### 1. `fn get_latency() -> Duration`
* **Descrição:** Mede o Round Trip Time (RTT) entre a origem e o servidor.
* **Lógica:** Dispara um pacote de controle (ICMP ou TCP Handshake) e utiliza a biblioteca `std::time::Instant` para capturar a diferença em microssegundos.
* **Contexto:** Essencial para o cálculo de Jitter em auditorias de rede.

### 2. `fn run_download_test(threads: usize) -> f64`
* **Descrição:** Gerencia o fluxo de recebimento de dados massivos.
* **Lógica:** Instancia múltiplos workers utilizando `std::thread`. Cada thread abre uma conexão de stream e lê buffers de dados (geralmente 8KB) em loop.
* **Diferencial:** Utiliza tipos atômicos (`std::sync::atomic`) para somar o total de bytes recebidos globalmente sem causar *race conditions* ou travar a CPU.

### 3. `fn run_upload_test(threads: usize) -> f64`
* **Descrição:** Mede a capacidade de saída da interface.
* **Lógica:** Similar ao download, mas foca na escrita de buffers aleatórios (`std::io::Write`) para o servidor. 
* **Performance:** Otimizado para não saturar a memória RAM do dispositivo (importante para uso no Termux), enviando pedaços de dados diretamente do buffer de saída.

### 4. `fn calculate_results(total_bytes: u64, duration: Duration) -> f64`
* **Descrição:** Converte os dados brutos para unidades legíveis (Mbps/Gbps).
* **Cálculo:** `(total_bytes * 8) / duration.as_secs_f64() / 1_000_000`.
* **Precisão:** Trabalha com ponto flutuante de 64 bits para garantir que medições em redes de alta velocidade (como as que você via na Logicalis ou Avanade) sejam exatas.

### 5. `fn handle_signals()`
* **Descrição:** Captura interrupções do sistema (Ctrl+C).
* **Lógica:** Garante que, se o usuário interromper o teste, todos os sockets e threads sejam encerrados graciosamente, evitando que processos fiquem "pendurados" no SO (zombie threads).

---

## 🛠 Troubleshooting e Diagnóstico

### Erros de Socket (Error 101/111)
Geralmente causados por bloqueio de firewall ou servidor offline. O programa reporta o erro via `std::io::Error` para facilitar o debug via `strace`.

### Saturação de CPU no Termux
Se o número de threads for superior à capacidade do processador do celular, o Jitter aumentará. Recomenda-se usar `n-1` threads em relação aos cores do dispositivo para manter a estabilidade do SO.

---
*Documentação gerada para o portfólio técnico de Kleber de Souza Nicolau.*
