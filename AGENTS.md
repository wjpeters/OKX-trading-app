# OKX Tradingterminal

Nieuw project, expliciet alleen lokaal op 2026-10-02. Geen code, configuratie, sleutels of ontwerp uit eerdere tradingprojecten overnemen. BUILD_SPEC.md is de volledige productscope. Gebruikerskeuze lokaal heeft voorrang op de VPS-optie in de specificatie.

- Gebruik rtk / rtk proxy voor shellcommando’s.
- Jev geeft voorstellen. Alleen een vaste deterministische risicokern mag uitvoerbare intents uitgeven aan de enige execution-gateway. Geen browser-, script- of AI-orderpad.
- Actuele release: PAPER_LOCAL met Pine, Bot Studio, alerts, Jev-voorstellen, nieuws en evaluatie. Lees STATUS.md en docs/PHASE3_4.md. Private protocollen zitten uitsluitend in de afzonderlijke Demo-onboardingbinary; de paperterminal importeert exchange.rs niet. Geen gebruikersrekening, budget, key, botstart of betaalde Jev-call namens Willem instellen. Testorders horen in disposable testdatabases.
- Het verzoek is de volledige bouwprompt af te werken. Ontbrekende keys blokkeren echte provideracceptatie, niet onafhankelijke ontwikkeling. Documenteer ontbrekende implementatie afzonderlijk van ontbrekende acceptatie.
- LIVE blijft uit tot expliciete account-, instrument-, budget- en limietbevestiging plus volledige technische acceptatie.
- Geen secrets in Git, logs, AI-input, frontend of images.
- Houd STATUS.md en docs/ACCEPTANCE.md eerlijk. Geen mocks presenteren als werkende functies.
