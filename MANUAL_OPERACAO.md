# Manual de Operação - Speedtest-RS Auditor (Engine 64KB)

## 1. Visão Geral
O Speedtest-RS é uma ferramenta de auditoria de rede desenvolvida em Rust, otimizada para o seu Cyberdeck e ambientes de alto desempenho (como o Termux). O motor utiliza buffers rígidos de 64KB para garantir a medição da taxa de transferência real.

## 2. Modos de Operação

### A. Interface de Auditoria Permanente (TUI)
Este é o modo principal para uso visual. Ele exibe os testes de Download e Upload de forma sequencial e mantém os resultados fixos na tela para análise posterior.
Comando:
./target/release/speedtest_rs --tui

* Comportamento: A interface entra em modo "fullscreen" e permanece ativa mesmo após o fim dos testes.
* Controle: O programa só será encerrado quando você pressionar Ctrl + C.

### B. Modo Auditoria Técnica (JSON)
Gera uma saída estruturada em JSON, ideal para documentar logs de rede ou automatizar relatórios de desempenho. Inclui timestamp, nome do host e velocidades precisas.
Comando:
./target/release/speedtest_rs --json

### C. Modo CLI Padrão
Execução convencional via linha de comando com barras de progresso simples.
Comando:
./target/release/speedtest_rs

## 3. Comandos e Parâmetros
-d, --duration: Tempo de duração para cada teste (Download/Upload) em segundos. Padrao: 10.
--tui: Ativa a interface visual permanente estilo Cyberdeck.
--json: Exporta o resultado consolidado em formato JSON.
--help: Exibe o menu de ajuda e versão.

## 4. Especificações Técnicas do Motor
- Buffer de Leitura: 64 KB (Rígido, otimizado para o Kernel Linux).
- Paralelismo: Threads automáticas baseadas nos núcleos da CPU (máximo de 8).
- Precisão: Cálculos baseados em tempo real com amostragem a cada 100ms na TUI.
- Protocolo: HTTP/1.1 para simulação de tráfego real de aplicação.

## 5. Atalhos e Teclas (Modo TUI)
- Ctrl + C: Finaliza o processo com segurança e restaura o terminal original.

## 6. Compilação (Otimizada)
Sempre utilize o perfil de release:
cargo build --release
