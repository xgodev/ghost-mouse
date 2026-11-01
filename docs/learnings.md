---
tags: [ghost-mouse, learnings]
created: 2026-10-31
updated: 2026-11-01
source: claude-code-sessions
---

# ghost-mouse — Learnings

## 2026-10-31 — Clique em página web dentro de janela que não está na frente

- **Gotcha / invariant:** numa janela sem foco, o AppKit pergunta `acceptsFirstMouse`; o WKWebView responde falso e o `mouseDown` não chega à página. `AXPress` no elemento da página funciona atrás de outros apps, sem mexer no cursor nem trazer a janela para a frente.
- **Why it matters:** evento por `CGEvent.postToPid` não torna a janela ativa, então clique por evento nunca aciona esse conteúdo.
- **Applies to:** a árvore de acessibilidade do web view só carrega depois de consultada (cerca de 1 a 2 s), e a posição logo após `open` pode vir defasada: não comparar frames com igualdade exata. Coordenada de acessibilidade é de tela (ponto da janela + x,y da janela).

## 2026-11-01 — Contador de apps do macOS passa de 32 bits

- **Gotcha / invariant:** o número de série (ASN) de cada programa sobe 0x1001 a cada programa registrado. Ao passar de `0x0-0xffffffff` vira `0x1-…`; a partir daí o `loginwindow` registra `Unable to get PID for context [1,…]: status -600`, o que abre por LaunchServices fica em estado `T` em `_dyld_start`, e `postToPid`/`postToPSN` não entregam. `screencapture` e `AXPress` continuam funcionando.
- **Why it matters:** o sintoma parece permissão ou bug do programa; `lsappinfo info -only ASN <pid>` mostra a causa em um comando. Reiniciar zera a contagem (medido: ASN volta a `0x0-…` e a Calculadora abre, recebe tecla e aparece no `shot`).
- **Applies to:** quem chama `osascript` em laço gasta o contador (uma chamada por segundo estoura em cerca de 21 dias).

## 2026-11-01 — Lista que o programa abre fica fora da tela se ele não está na frente

- **Gotcha / invariant:** painel com `hidesOnDeactivate` (o `wxPopupTransientWindow` dos combos do wxWidgets, por exemplo) recebe o `orderFront`, o AppKit diz que está visível, mas o window server o mantém com `kCGWindowIsOnscreen` falso enquanto o programa não está na frente. Ele fica igual aos painéis nunca abertos na lista pública; só o bit `0x2000` de `SLSGetWindowTags` separa (aberto `1000c6002`, fechado `1000c4002`, nunca aberto `1000c0002`). `SLSWindowIsOrderedIn` devolve 0 e não serve.
- **Why it matters:** o clique chega e a lista abre, só que ninguém vê, e parece que o clique falhou. `screencapture -l<id>` e evento com o número da janela funcionam nela mesmo fora da tela.
- **Applies to:** botão desenhado pelo próprio programa (ImGui em GL) só aceita a pressão com o ponteiro parado antes: 30 e 100 ms erraram, 200 ms acertou. `AXPress` não serve em painel flutuante: a acessibilidade escolhe a janela mais próxima e apertaria a página embaixo dele.
