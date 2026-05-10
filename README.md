# SpeedTest-RS 🚀

Um utilitário de medição de velocidade de rede de alta performance desenvolvido em **Rust**, focado em precisão, baixo consumo de recursos e concorrência segura.

Este projeto foi concebido para operar em ambientes diversos, desde terminais mobile (como Termux) até servidores de infraestrutura crítica, utilizando o modelo de concorrência de Rust para garantir medições estáveis sem comprometer a integridade do sistema.

## 🛠 Habilidades Técnicas Aplicadas
- **Linguagem:** Rust (foco em segurança de memória e performance "bare metal").
- **Concorrência:** Uso de Threads e Atômicos para processamento paralelo massivo.
- **Redes:** Manipulação de sockets e análise de latência em nível de pacote (ICMP/TCP).
- **Arquitetura:** Design focado em baixo nível, ideal para integração com Kernels e sistemas operacionais.

## 📋 Funcionalidades
- Medição de Download e Upload em tempo real com buffers otimizados.
- Cálculo preciso de Latência (Ping) e Jitter.
- Relatórios detalhados via Interface de Linha de Comando (CLI).
- Otimizado para execução em ambientes restritos (Android via Termux).

## 🚀 Como Executar
1. Certifique-se de ter o ambiente Rust instalado.
2. Clone o repositório: 
   `git clone https://github.com/kleber-nicolau/speedtest-rs.git`
3. Entre na pasta: 
   `cd speedtest-rs`
4. Compile e execute: 
   `cargo run --release`

---
*Este projeto faz parte do meu portfólio de Engenharia de Sistemas e Cibersegurança.*
