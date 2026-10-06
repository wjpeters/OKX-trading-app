# OKX Tradingterminal

Nieuw project, expliciet alleen lokaal op 2026-10-02. Geen code, configuratie, sleutels of ontwerp uit eerdere tradingprojecten overnemen. BUILD_SPEC.md is de volledige productscope. Gebruikerskeuze lokaal heeft voorrang op de VPS-optie in de specificatie.

- Gebruik rtk / rtk proxy voor shellcommando’s.
- Jev geeft voorstellen. Alleen een vaste deterministische risicokern mag uitvoerbare intents uitgeven aan de enige execution-gateway. Geen browser-, script- of AI-orderpad.
- Actuele release: PAPER_LOCAL met Pine, Bot Studio, alerts, Jev-voorstellen, nieuws en evaluatie. Lees STATUS.md en docs/PHASE3_4.md. Private protocollen zitten uitsluitend in de afzonderlijke Demo-onboardingbinary en de native ordergateway; de paperterminal importeert exchange.rs niet. Handmatige Demo- en Live-routes zijn gebouwd. Willem vraagt op 2026-10-05 expliciet handmatige Live-vrijgave; die vraagt zijn eigen account-/markt-/budget-/limietkeuze en persoonlijke vrijgave. Echte native orderacceptatie en volledige fase 5 blijven open. Zie docs/ORDERS.md. Geen gebruikersrekening, budget, key, botstart of betaalde Jev-call namens Willem instellen. Testorders horen in disposable testdatabases.
- Het verzoek is de volledige bouwprompt af te werken. Ontbrekende keys blokkeren echte provideracceptatie, niet onafhankelijke ontwikkeling. Documenteer ontbrekende implementatie afzonderlijk van ontbrekende acceptatie.
- Handmatige LIVE-orders vereisen expliciete account-, instrument-, budget- en limietbevestiging, een afzonderlijke persoonlijke vrijgave en actuele private verbinding/reconciliatie. Dit is op 2026-10-05 door Willem verzocht; maak geen Live-botvrijgave of volledige technische acceptatie van deze uitzondering. Er worden geen persoonlijke bedragen of testorders in zijn rekening ingesteld.
- Geen secrets in Git, logs, AI-input, frontend of images.
- Houd STATUS.md en docs/ACCEPTANCE.md eerlijk. Geen mocks presenteren als werkende functies.

- Op 2026-10-05 vraagt Willem expliciet automatisch ingevulde, aanpasbare voorbeeldlimieten na invoer van zijn handelsbudget. Dit mag uitsluitend in het formulierconcept. Persoonlijke bevestiging blijft vereist voor opslaan; er wordt geen accountbeleid of Live-vrijgave automatisch aangepast. Voorbeelden zijn geen statistisch gemiddelde van traders.

- Op 2026-10-06 geeft Willem expliciet toestemming voor handmatige Live-ondersteuning op alle EUR- en USDC-spotmarkten die zijn OKX-account toestaat, inclusief de daarvoor noodzakelijke gatewaybijwerking. Bestaande persoonlijke budgetten, limieten en marktkeuze blijven behouden; nieuwe markten vragen eigen bevestiging en daarop gebonden Live-vrijgave. Geen echte testorders, transfers, andere betaalvaluta’s, margin/futures of Live-botvrijgave.
