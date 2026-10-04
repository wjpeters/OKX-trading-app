# Providers, licenties en verificatie · 2026-10-02

Geen licentieaankoop, abonnement, voorwaardenacceptatie namens de gebruiker of betaalde providercall uitgevoerd. Een documentatiecontrole bewijst geen accountrechten. Links naar primaire bronnen; onbekende details blokkeren uitsluitend de betrokken integratie.

| Component | Bevinding / keuze | Status |
|---|---|---|
| OKX EEA V5 | Native publieke API, SPOT-catalogus; host eea.okx.com, wseea.okx.com | Publieke calls getest; persoonlijke regio/rechten onbevestigd |
| Lightweight Charts 5.2.1 | Apache-2.0 plus NOTICE en TradingView-link; ingebouwde attributionLogo aan | Gebruikt, NOTICE hieronder en UI-link |
| Advanced Charts / Trading Platform | Geen aanname dat privégebruik en Pine-runtime worden geleverd | Niet gebruikt |
| PineTS | Repository vermeldt AGPL-3.0 / commerciële optie; vervangbare runtimekandidaat | PineTS0.11.0 lokaal geïntegreerd, parser/AST-profiel getest; licentiebeoordeling vereist vóór distributie |
| DeMARK Sequential/Combo | Proprietaire indicatorfamilie; geen officiële implementatiespecificatie/licentie vastgesteld | Niet geïmplementeerd; geen 9/13-teller onder officiële naam |
| Jev | OpenRouter Decisions API, model typesafe/jev-1.13; state/questions en getypte Choice/Noul/Score | Decisions-adapter en responsefixtures getest; geen key of echte betaalde call |
| CryptoPanic | Publieke documentatiepagina onvoldoende uitleesbaar voor exact plan/schema/quota | Geblokkeerd tot providerrechten en actuele documentatie vastgesteld |
| CoinGecko | Demo/keyless endpointoverzicht beschikbaar; plan bepaalt dekking/quota | Alleen onderzocht, geen sleutel/adapter |
| Alternative.me | Dagelijkse Bitcoin-index, geen realtime altcoin-sentiment | Dagelijkse BTC-index werkelijk geïntegreerd; bronvermelding in UI |
| React / Vite / Tailwind / Zustand / TanStack / Rust-crates | Gepinde packages, licenties in distributies | Dependencies geïnstalleerd; volledige SBOM-review vóór trading |

## Primaire bronnen

- [OKX EEA](https://my.okx.com/docs-v5/en/)
- [Poort 8443 sluit 31 oktober 2026; 443 beschikbaar](https://www.okx.com/en-eu/help/okx-websocket-port-8443-discontinuation-announcement)
- [Checksum-deprecatie; books5 unaffected](https://www.okx.com/help/okx-order-book-channels-checksum-field-deprecation)
- [Chartbibliotheken](https://www.tradingview.com/free-charting-libraries/)
- [Lightweight Charts](https://github.com/tradingview/lightweight-charts)
- [PineTS](https://github.com/LuxAlgo/PineTS)
- [DeMARK Sequential](https://demark.com/sequential-indicator/)
- [Jev integratie](https://openrouter.ai/docs/guides/community/jev)
- [Decisions schema](https://openrouter.ai/docs/api/api-reference/alphadecisions/submit-a-decisions-request)
- [TypeSafe confidence](https://docs.typesafe.ai/confidence)
- [CryptoPanic](https://cryptopanic.com/developers/api/)
- [CoinGecko endpoints](https://docs.coingecko.com/demo/reference/endpoint-overview)
- [Alternative.me](https://alternative.me/crypto/fear-and-greed-index/)

## TradingView NOTICE

TradingView Lightweight Charts™
Copyright (с) 2025 TradingView, Inc. https://www.tradingview.com/

Gebruikte versie: 5.2.1. Exacte NOTICE gecontroleerd op de bijbehorende Git-tag. Apache-2.0 LICENSE is meegeleverd in het npm-package. De chart toont de TradingView-attributie en een zichtbare link. Dit product is een onafhankelijke privéterminal.

## Jev datacontract

De interne DecisionEnvelope is geen providerresponse. De adapter normaliseert het gedocumenteerde responseschema, en neemt probabilities/confidence/kosten uitsluitend uit de ontvangen response over. Ontbrekende/ongeldige velden blokkeren AI-afhankelijke exposure. Geen gewone chatcompletion, geen latest-alias of automatische providerfallback. Confidence is geen bewezen winstkans. Geen beslissingen zonder ingestelde kostenlimiet.

OKX-announcements en status worden als aparte langzame adapters opgehaald. Bij de regionale acceptatie was de announcementslijst leeg; geen vervangende nieuwstekst verzonnen. Monaco en React Flow zijn op exacte packageversies gelockt. Rust-advisories en images worden lokaal gescand; zie evidence en THREAT_MODEL.md.
