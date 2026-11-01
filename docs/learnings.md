---
tags: [ghost-mouse, learnings]
created: 2026-10-31
updated: 2026-10-31
source: claude-code-sessions
---

# ghost-mouse — Learnings

## 2026-10-31 — Clique em página web dentro de janela que não está na frente

- **Gotcha / invariant:** numa janela sem foco, o AppKit pergunta `acceptsFirstMouse`; o WKWebView responde falso e o `mouseDown` não chega à página. `AXPress` no elemento da página funciona atrás de outros apps, sem mexer no cursor nem trazer a janela para a frente.
- **Why it matters:** evento por `CGEvent.postToPid` não torna a janela ativa, então clique por evento nunca aciona esse conteúdo.
- **Applies to:** a árvore de acessibilidade do web view só carrega depois de consultada (cerca de 1 a 2 s), e a posição logo após `open` pode vir defasada: não comparar frames com igualdade exata. Coordenada de acessibilidade é de tela (ponto da janela + x,y da janela).
