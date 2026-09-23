# Nitro Control — Especificação de design

Data: 2026-09-23
Máquina alvo: Acer Nitro AN16-51, BIOS V1.12, Ubuntu 26.04 (GNOME), kernel 7.0
Base de hardware: ASense 0.3.0 (`asensed` + módulo `asense_rgb` + `acer_wmi`)

## 1. Objetivo

Um painel de controle no estilo NitroSense (escuro, com brilho colorido e cantos
chanfrados) que **substitui por completo a interface do ASense**. O daemon
`asensed` continua como motor por baixo. O app precisa de:

- visual próximo do NitroSense, recriado do zero, sem usar arte, logos ou
  imagens da Acer;
- acesso rápido: ícone na barra do GNOME com as temperaturas e um menu rápido;
- temperaturas de CPU e GPU sempre à vista, na barra e na janela;
- troca de modo com animação e cor própria de cada modo;
- sincronização com o hardware: se o modo mudar fora do app, a interface
  acompanha.

### Fora do escopo

- Curva de ventoinha personalizada e modo manual. O daemon aceita
  `FAN MANUAL`, mas o usuário disse que Auto e Máximo bastam.
- RPM das ventoinhas, real ou estimado. Mostra só a porcentagem de rotação
  (decisão do usuário).
- Efeitos de teclado além dos que o daemon aceita (onda, zoom etc.).
- Histórico persistente do monitor.
- Tuning da NVIDIA e outros recursos exclusivos do Predator PHN16-72.

## 2. Contexto técnico levantado

- Socket `/run/asense-control.sock`, `0600`, dono `vilefort`. O daemon atende
  **uma conexão de controle por vez**. O primeiro comando precisa ser
  `HELLO 2`. Cada resposta é `OK <payload>` ou `ERR <mensagem>`. Comandos têm
  até 192 bytes e respostas até 4096 bytes. Um `ERR` rejeita só aquele comando
  e a sessão continua usável.
- Comandos usados:
  - descoberta: `PING`, `CAPS` (JSON de capacidades), `HARDWARE GET`,
    `PLATFORM GET`;
  - perfil: `PROFILE <token-de-CAPS>`. O probe listou os tokens `low-power`
    (eco), `quiet`, `balanced`, `balanced-performance` (desempenho) e
    `performance` (turbo). Os tokens valem como vierem do `CAPS`, não
    ficam fixos no código;
  - ventoinha: `FAN AUTO`, `FAN MAXIMUM`;
  - iluminação: `LIGHTING APPLY <device-id> <OFF|STATIC|BREATHING|NEON>
    <brilho 0..100> <velocidade 0..9> <RRGGBB> <-|RRGGBB,...>` e
    `LIGHTING POWER <device-id> <ON|OFF>`. O teclado é 4 zonas
    (`zone_mask 0x0f`);
  - plataforma: `PLATFORM <BATTERY_LIMIT|KEYBOARD_TIMEOUT|BOOT_SOUND|LCD_OVERRIDE> <ON|OFF>`,
    `PLATFORM BATTERY_CALIBRATION <START|STOP>` e
    `PLATFORM USB_CHARGING <0|10|20|30>`.
- O formato exato das respostas de `HARDWARE GET`, `PLATFORM GET` e `CAPS` é
  capturado com o daemon real na primeira tarefa de implementação e vira
  fixture dos testes.
- Leitura do ASense neste modelo: perfil ok, porcentagem das ventoinhas ok,
  RPM e temperaturas não disponíveis.
- Sensores fora do ASense:
  - CPU: hwmon `coretemp`, `temp1_input`, pacote;
  - sistema: hwmon `acpitz`;
  - SSD: hwmon `nvme`;
  - GPU: NVIDIA (temperatura, uso, frequência e energia) via NVML, com
    `nvidia-smi` como alternativa. Antes de ler, o app confere
    `/sys/bus/pci/devices/<gpu>/power/runtime_status`: se não for `active`,
    mostra "em repouso" e não acorda a GPU;
  - uso de CPU: `/proc/stat`; RAM: `/proc/meminfo`.

## 3. Arquitetura

Um único programa, `nitro-control`, feito em Tauri 2: backend em Rust e
interface em HTML/CSS/JS com o WebKitGTK do sistema. Ele inicia com o login e
fica rodando sempre. Fechar a janela só a esconde.

| Módulo (Rust) | Responsabilidade | Depende de |
|---|---|---|
| `asense` | Única conexão com o socket: handshake, envio de comandos, parse de `OK`/`ERR` e reconexão com espera de 2 s | socket |
| `sensors` | Lê coretemp, acpitz, nvme, GPU, `/proc/stat` e `/proc/meminfo`. Leitores com caminho-base injetável, para teste | `/sys`, `/proc`, NVML |
| `state` | Guarda o estado central (modo, ventoinha, iluminação, plataforma, sensores e conexão), calcula diferenças e publica eventos | `asense`, `sensors` |
| `automation` | Aplica a cor do modo no teclado quando o modo muda (se "Acompanhar o modo" estiver ligado) e aplica os padrões do login | `state`, `asense`, `config` |
| `config` | Lê e grava `~/.config/nitro-control/config.toml` | fs |
| `tray` | Ícone e menu na barra (StatusNotifierItem, via o suporte a bandeja do Tauri) | `state` |
| `commands` | Comandos Tauri expostos à interface: `set_profile`, `set_fan`, `set_lighting`, `set_platform`, `get_snapshot`, `save_config` | `asense`, `state`, `config` |

A interface (`ui/`) só recebe eventos do backend e chama comandos. Ela nunca
fala com o socket nem com o `/sys`.

### Ciclos

- Sensores: a cada 1 s com a janela visível e a cada 2 s com ela escondida
  (só o ícone consome).
- Estado do ASense: `HARDWARE GET` a cada 2 s.
- Eventos para a interface só quando algo muda, e só com a janela visível.

## 4. Fluxo de dados

1. Um clique num modo chama o comando `set_profile(token)`, que envia
   `PROFILE token`.
2. Com `OK`, o backend relê o estado na hora, sem esperar o próximo ciclo.
3. O `state` detecta a mudança de modo e publica `mode-changed`.
4. A interface troca a paleta (transição de cerca de 0,9 s) e roda a animação
   de troca. O `automation` aplica a cor do modo no teclado.
5. Uma mudança feita fora do app entra pelo ciclo de 2 s e segue os passos 3
   e 4 do mesmo jeito.

A interface só reflete o que o hardware confirmou: não há atualização otimista.

## 5. Visual

Referência: NitroSense (foto enviada pelo usuário). Os elementos são recriados
em CSS e SVG:

- fundo em degradê radial escuro, tingido pela cor do modo;
- painéis com cantos chanfrados (`clip-path`), borda fina e um filete
  luminoso no topo;
- medidores circulares (`conic-gradient` ou SVG) com brilho externo;
- botões de modo em paralelogramo;
- abas no topo: **Início · Desempenho · Teclado · Monitor · Sistema**.

### Paleta por modo (editável na aba Teclado)

| Modo | Cor principal | Cor secundária |
|---|---|---|
| Eco | `#22c55e` verde | `#86efac` |
| Silencioso | `#38bdf8` azul | `#a5e3ff` |
| Equilibrado | `#ff8a1f` laranja | `#ffc07a` |
| Desempenho | `#ff2a1a` vermelho | `#ff6a2b` |
| Turbo | `#b026ff` roxo | `#ff3df2` magenta |

O Turbo tem brilho mais forte e animação mais intensa.

### Animação de troca de modo

Um feixe de luz varre a janela (~0,9 s), os painéis dão um pulso de brilho e o
nome do modo aparece grande no centro (~1,3 s). No Turbo, o nome fica maior, o
brilho mais forte e a animação dura ~1,6 s. A animação respeita
`prefers-reduced-motion`: nesse caso, só a troca de cor acontece.

### Ventoinhas

Duas hélices em SVG (CPU e GPU) que giram mais rápido quanto maior a
porcentagem, com o número em % e uma barra.

## 6. Abas

**Início**
- esquerda: medidor grande da GPU (frequência) com uso da GPU, uso da CPU e
  energia da GPU;
- centro: título, modo atual, os 5 modos, ventoinha Auto/Máximo e prévia das
  4 zonas do teclado;
- três medidores de temperatura empilhados: GPU, CPU e sistema;
- coluna direita: perfil ativo (modo, ventoinha, efeito do teclado e
  bateria) e grade de monitoramento (uso e temperatura de CPU e GPU, RAM e
  SSD).

**Desempenho**: os 5 modos em cartões grandes na cor de cada um, as duas
hélices com % e os botões Auto e Máximo.

**Teclado**
- efeito: Desligado, Estático, Respiração ou Neon;
- brilho de 0 a 100 e velocidade de 0 a 9, esta só em Respiração e Neon;
- chave "Acompanhar o modo", ligada por padrão, que põe as 4 zonas na cor
  principal do modo;
- com a chave desligada, seletor de cor para cada uma das 4 zonas;
- editor da paleta dos 5 modos, que vale para a janela e para o teclado.

**Monitor**: gráficos dos últimos 5 minutos (temperatura de CPU e GPU, uso de
CPU e GPU, energia da GPU e % das ventoinhas). O histórico fica numa fila
circular em memória.

**Sistema**
- limite de bateria em 80%, calibração da bateria (iniciar/parar), carga USB
  desligado (0/10/20/30), som de boot e tempo para apagar a luz do teclado;
- padrões do login: modo inicial (padrão Turbo) e ventoinha inicial (padrão
  Auto);
- chave "Iniciar com o sistema".

**Menu do ícone na barra**: o rótulo mostra `CPU° · GPU°`. O menu tem as
temperaturas, os 5 modos (com o atual marcado), Ventoinha Auto/Máximo e
"Abrir painel".

## 7. Tratamento de erros

| Situação | Comportamento |
|---|---|
| `ERR <msg>` num comando | Aviso rápido na janela com a mensagem; o controle volta ao estado real |
| Socket caiu ou daemon reiniciou | Reconecta a cada 2 s; a janela mostra "ASense desconectado", desativa os controles e mantém os sensores |
| Conexão ocupada por outro cliente | Aviso: "A interface do ASense está aberta — feche-a para usar o Nitro Control" |
| GPU em repouso | Medidor da GPU mostra "em repouso", sem acordar a GPU |
| Sensor ausente ou ilegível | Aquele valor mostra "—"; o resto continua |
| Config inválida ou ausente | Usa os padrões, registra aviso no log e regrava a config só quando o usuário salvar algo |

O log vai para o journal (stderr) com nível configurável por `RUST_LOG`.

## 8. Testes

- `asense`: um daemon falso (socket Unix temporário) cobre handshake, `OK`,
  `ERR`, resposta malformada, queda e reconexão.
- `sensors`: árvores de `/sys` falsas em diretório temporário cobrem
  coretemp, acpitz, nvme, GPU `active` e `suspended` e sensor ausente.
- `state` e `automation`: uma mudança de modo externa gera `mode-changed` e
  aplica a cor certa; com "Acompanhar o modo" desligado, o teclado não é
  alterado.
- `config`: leitura, padrões e arquivo inválido.
- Interface: modo demonstração (`?demo=1` em dev) com dados falsos, para
  conferir visual e animações no navegador.
- Teste no hardware real, no fim:
  - trocar os 5 modos e conferir com `asense probe --summary`;
  - alternar Auto/Máximo;
  - conferir as cores do teclado por modo;
  - mudar o modo por fora e ver o app acompanhar;
  - derrubar e subir o daemon e ver a reconexão.

## 9. Instalação e migração

1. Uma vez, via `pkexec apt install`: `libwebkit2gtk-4.1-dev`,
   `libayatana-appindicator3-dev`, `librsvg2-dev` e `build-essential`.
2. `cargo tauri build` gera o `.deb`. Ele instala `/usr/bin/nitro-control`,
   um `.desktop` e um ícone próprio (sem arte da Acer).
3. Script `migrar.sh`, com backup e `reverter.sh`:
   - remove `~/.config/autostart/asense_turbo_fans.desktop` e
     `rgb_config_acer_gkbbl_0.desktop`;
   - impede que a interface do ASense abra no login;
   - aponta o atalho customizado do GNOME (`.../custom-keybindings/asense/`)
     para `nitro-control --toggle`;
   - instala o autostart do `nitro-control`.
4. `nitro-control --toggle` mostra ou esconde a janela da instância que já
   está rodando (instância única).

Código em `~/nitro-control`, versionado com git.
