## Update 6 oktober 2026 · draagbare netwerkexport en import

Training Lab bevat Netwerkarchief: JSON-export en persistente import van vastgezette kandidaten, het nieuwste tussentijdse netwerk en eerdere generatiecheckpoints. Alle 340 gewichten/biases, normalisatie, parameters en model-/dataversies blijven behouden. Export kan na de eerste berekende generatie, vóór volledige afronding. Import is een onderzoekskopie zonder handelsvrijgave of overgenomen testbewijzen; volledige optimizerhervatting blijft in de oorspronkelijke campagne.

254 Rust-tests, TypeScript/productiebuild en 20 geïsoleerde portabilitycontroles slagen. De browserproef controleert werkelijke bestandsdownloads, een opgeslagen netwerk terugladen, een eerdere generatie en de matrices. Gewijzigde bestanden zijn via de API afgewezen. De aanvullende uploadproef via de browser is niet uitgevoerd: de uploadtoestemming werd geweigerd; er is geen omweg geprobeerd. De testgegevens zijn kunstmatig, uitsluitend in een wegwerpdatabase. Alleen de terminal is bijgewerkt; beide gateways, trainer, rekeningpolicy en persoonlijke vrijgave zijn behouden. [Bediening](docs/TRAINING_LAB.md#netwerk-exporteren-en-importeren), [bestands-/databaseproef](evidence/training-transfer-integration.json), [behoudcontrole](evidence/training-transfer-local.json).

## Update 6 oktober 2026 · Training Lab met neuro-evolutie

De lokale terminal heeft nu Training Lab in Bot Studio: 16/16/4-netwerken met echte gewichten en activaties, generatiecheckpoints, causale Rust-strategiekern, CMA-ES, gepinde lokale Laya, Parquet-datasetimports, eenmalige holdoutfamilies, Holm/bootstrap/kostenstress en afzonderlijke forward-paperadministratie. 248 Rust-tests, zes Python-tests, twintig onderzoekscontroles en achttien native botcontroles slagen. BTC/ETH-twaalfmaandsdatasets zijn compleet: ieder 525.600 candles, 100% coverage en geen gaten. De eerste campagne berekent Laya-regimes. Geen gebruikersbudget, livevrijgave of order namens Willem gewijzigd.

Bij de eerdere controle stond de campagne gepauzeerd na 51.216 Laya-featureberekeningen. Bij de gatewaycontrole om 14:39 draait zij weer met 72.816 featureberekeningen; de agent heeft haar niet hervat. Trainer en Laya zijn online. Dit is een momentopname; er is nog geen getrainde eindkandidaat.

**De volledige neurobotketen is nog niet geaccepteerd:** de bron bevat nu gezamenlijke rekeningreserveringen, eigen botinventaris en verliesgrenzen, native bescherming en vaste houdduurexits, persoonlijke exacte-versievrijgave en getoetste duurzame voorstellen. Echte Demo-acceptatie, volledige resource-/latencyduurproef en minimaal 30 kalenderdagen/100 forwardtrades blijven vereist. De botvrijgave blijft zonder die bewijzen geblokkeerd. Geen gekwalificeerde of winstgevende bot geclaimd. [Bediening en actuele grenzen](docs/TRAINING_LAB.md), [volledige opdracht](docs/TRAINING_LAB_SPEC.md).

Na expliciete toestemming van Willem zijn op 6 oktober om 14:39 beide bestaande Demo- en Live-gateways vervangen door de gebouwde en geteste versie. De lopende gateways en terminal hebben dezelfde uitvoeringscodehash. Budgetten, limieten, persoonlijke Live-vrijgave en order-/fillaantallen zijn behouden; terminal, trainer en database zijn niet herstart. Demo mist nog sleutels; Live blijft gepauzeerd wegens OKX 50110 / IP-toelating. Geen neurobot vrijgegeven. [Installatie- en behoudbewijs](evidence/training-gateway-update.json).

## Update 6 oktober 2026 · EUR/USDC geïnstalleerd; OKX-IP-controle vereist

Willem heeft de uitbreiding naar alle EUR- en USDC-spotmarkten van zijn eigen OKX-account expliciet toegestaan. De actuele bron is gericht samengevoegd en de terminal plus beide afzonderlijke ordergateways zijn bijgewerkt. De eerdere grens van twaalf persoonlijk gekozen markten is verwijderd. Budget, limieten, acht gekozen EUR-markten en de bestaande persoonlijke Live-vrijgave zijn behouden; geen nieuwe markt of order is namens Willem bevestigd.

Het account-endpoint blijft de bron voor beschikbare markten. **Alle beschikbare spotmarkten kiezen** selecteert de volledige EUR/USDC-accountlijst in het formulierconcept. De gebruiker bevestigt vervolgens zelf de limieten en geeft Live opnieuw vrij voor die nieuwe versie. Prijsvelden, previews en kooppercentages gebruiken de werkelijke betaalvaluta. EUR-budget en risico gebruiken actuele marktconversies. Een ontbrekende/verouderde conversie of exacte marktfee blokkeert de aankoop. Kopen met USDC en verkopen van USDC delen dezelfde saldoreservering; crypto-inventaris en blootstelling worden over EUR/USDC-paren van dezelfde munt samengenomen.

**Actuele accountcontrole is nog geblokkeerd:** OKX antwoordt HTTP 401 / code 50110 op `/api/v5/account/bills`, en de private login wordt eveneens geweigerd. De terminal toont dat het huidige IP-adres niet door de API-key wordt toegelaten. De IP-lijst moet door Willem bij OKX worden gecontroleerd. Er is geen verse EUR/USDC-accountlijst geclaimd: de oude lijst van 270 EUR-markten is verouderd. De gateway blijft gepauzeerd en de UI toont geen onjuiste marktweigering of actieve Live-verbinding op basis van dat oude snapshot.

Bewijs: productiebuild en TypeScript; 211 Rust-tests (32 onboarding, 44 gateway, 135 terminal); 32 bestaande native HTTP-controles en 25 aanvullende EUR/USDC-controles in een wegwerpdatabase. De tweede productiebrowserproef bevestigt USDC-prijsunits, grafiekklik met hoeveelheidbehoud, de IP-melding, en geen consolefouten. De fictieve browserrekening bevestigt wissen/alle kiezen, USDC-zoeken en uitgeschakelde opslag zonder eigen bevestiging. De sliders blijven in de browser uitgeschakeld zonder geconfigureerde keys; hun 50%-USDC-berekening is via de geïsoleerde HTTP-route getest. Geen echte testorder, transfer of gebruikersinstelling gewijzigd. Dit is geen volledige native exchange- of fase-5-acceptatie.

Bewijsbestanden: `evidence/eur-usdc-*`. Herhaalbare controles: `scripts/order_acceptance.py provision`, `scripts/eur_usdc_checks.py`, daarna `scripts/order_acceptance.py cleanup`. De opname verwijst uitsluitend naar de wegwerpomgeving op poort 4781.

## Update 5 oktober 2026 · Markttoegang en diagnose

De volledige publieke spotcatalogus is doorzoekbaar en extra grafieken krijgen gevalideerde live boekdata. Het ticket toont ontbrekende persoonlijke EUR-marktkeuze en de terminalbeperking op andere betaalvaluta's. De handelsroute blijft EUR-only, maximaal twaalf persoonlijk gekozen markten. De automatische controle heeft Live-uitbreiding en een diagnostische gatewayherstart geweigerd; alleen de terminal is vernieuwd. Het afzonderlijke EUR/USDC-voorstel heeft 43 geïsoleerde gatewaytests en typechecks, maar is niet geïnstalleerd. Budget, acht bevestigde markten en Live-vrijgave zijn behouden. Bij controle stond één volledig uitgevoerde native verkoop in de administratie; geen agentorder. Zie docs/MARKET_ACCESS_PROPOSAL.md en evidence/market-access-*.

## Update 5 oktober 2026 · Grafiekklik naar limietprijs

Een klik in het koerspaneel vult de actieve koop-/verkooplimietprijs in, afgerond op de marktprijsstap. Hoeveelheid en richting blijven behouden. Volume/indicatoren, tekenen en slepen wijzigen geen orderprijs. De functie werkt ook in meerdere grafieken en bij triggerlimietuitvoering. Een marktkeuze wist de oude limietprijs. Vier nieuwe prijs-/routingtests, zeven bestaande limiettests, elf tekenmodeltests en de TypeScript/productiebuild slagen; de productie-browserproef bevestigt het gedrag. Alleen de interface is lokaal vernieuwd. Geen rekeningbeleid, Live-vrijgave of orders door de agent gewijzigd. Bewijs en gebruik: docs/ORDERS.md en evidence/chart-price-*.

## Update 5 oktober 2026 · Slimme handelslimieten

Het limietenformulier vult na budgetinvoer aanpasbare voorbeeldwaarden in. Geldlimieten rekenen exact mee; eigen aanpassingen blijven behouden. Opgeslagen limieten worden geladen. Het startprofiel kan bewust opnieuw worden toegepast. Zeven gerichte tests, TypeScript/productiebuild en browserproef slagen. Tijdens de controle zijn via de bestaande bevestigingsroute limieten opgeslagen en is de persoonlijke Live-vrijgave actief geworden (limietversie 2). Er is niets automatisch opgeslagen en geen agentorder geplaatst. Dit verandert de open daadwerkelijke native orderacceptatie niet. Zie docs/ORDERS.md en evidence/limit-suggestions-*.

## Update 5 oktober 2026 · Live-saldi en percentages

Op expliciet verzoek is de handmatige Live-route beschikbaar gemaakt met eigen credentials/gateway, echte trading/funding-saldi en een serverberekende percentageslider. Live vraagt persoonlijk bevestigde account, markten, budget en limieten plus een afzonderlijke vrijgave. Elke order vraagt een exacte preview en bevestiging. Papierbots en Pine blijven paper. Volledige fase 5 en echte native orderacceptatie blijven open; de persoonlijke vrijgave verandert dat niet. Zie docs/ORDERS.md.

# Bouwstatus · 2 oktober 2026 · versie 0.4.0

**De lokale terminal heeft markt-/paper-/onderzoeksfuncties en native handmatige OKX Demo- en Live-orderroutes. Live vereist persoonlijke account- en limietvrijgave. De volledige bouwprompt en echte exchange-orderacceptatie zijn nog niet afgerond.** De volledige scope blijft staan in BUILD_SPEC.md; [REMAINING.md](docs/REMAINING.md) maakt ontbrekende implementatie en ontbrekende acceptatie afzonderlijk zichtbaar.

Open http://127.0.0.1:4780 via **Start OKX Terminal.command**. Alles draait lokaal, buiten iCloud. Bij de orderupdate zijn bestaande gebruikersrekening- en botinstellingen behouden. De afzonderlijke Demo-gateway wacht op keys; nul native exchangeorders geplaatst. Exchangeaccounts, handelsbudgetten, keys en livevrijgave zijn niet namens Willem gekozen.

## Werkend en getest

- **Marktterminal:** twaalf echte EUR/USDC-spotmarkten, `books` snapshot/delta met sequencecontrole, tape/tickers, veertien timeframes, tien instelbare native indicatoren, één tot vier gekoppelde charts, tijdzones, charttypen, fullscreen, command palette, workspaces en links/rechts te plaatsen panelen.
- **Tekenen en alerts:** negen tekenvormen, magneet, prijs-/tijdankers, numerieke ankerbewerking, lock, undo/redo en browserpersistentie. Serveralerts voor prijs/indicator/vastgelegde tekenlijn/bot/risico, cooldown, dedup en duurzame inbox. Lijnalerts projecteren echte ankers door de tijd.
- **Vaste paper-risk/OMS:** atomaire budget-/inventarisreservering, duurzame intents, aparte ACK/fill, partial fills, stop/target, dag-/totaalverlies/drawdown/HWM, account-/botlimieten, idempotentie, crash-/DB-herstel en één paperexecutor. Handmatig, bots en Pine volgen dezelfde kern.
- **Pine Editor:** echte PineTS 0.11-parser/AST in geïsoleerde netwerkloze worker, Monaco, inputformulieren met grenzen, diagnostiek, immutable versies, import/export, plots en Pine-candle-backtest. Gevalideerd closed-bar-profiel; geen volledige TradingView-semantiekclaim.
- **Bot Studio:** ReactFlow plus canonieke configuratie, servervalidatie van DAG en parameters, verplichte riskgate, trend/range/Jev-templates, clone/rollback en immutable deployment naar een gepauzeerde paperbot zonder exposure. Serverberekening en beschermingsloop blijven zonder browser draaien.
- **Historie/evaluatie:** hervatbare echte OKX-candleimports als Snappy-Parquet, manifest/coverage/download; walk-forward met training-only gevoeligheid en twee aparte OOS-vensters; opgeslagen rapporten terugladen/exporteren. Pine gebruikt dezelfde runtime en riskkern in replay/paper, next-barfills en conservatieve kosten/stopvolgorde.
- **Jev-providerlaag:** werkelijk Decisions-schema/modelpin, alleen voorstellen, schema/confidence/TTL/statechecks, concurrency, deadlines, kostencaps/reservering per rekening/bot-id en breaker. Fixtures slagen; echte provideracceptatie ontbreekt zonder key/budget. Vergelijkingsrapport controleert matching forward-paperdeployments met/zonder Jev, zonder verzonnen FX of achteraf modelcalls.
- **Nieuws/sentiment:** echte OKX announcements/status en dagelijkse Bitcoin Fear & Greed met bronstatus/tijden. Regionale nieuwsroute gaf bij controle nul berichten; geen headlines verzonnen. CryptoPanic/CoinGecko/DeMARK blijven zichtbaar beperkt.
- **Operatie:** optionele lokale login met sessieverval/CSRF, versleutelde providercredentials, diskbewaking, logrotatie, health/readiness/metrics, AES-GCM-backup en geteste geïsoleerde restore van DB plus Parquet. Non-root/minimale images met lokale securityscans.

## Uitgevoerde controles

| Controle | Uitkomst | Bewijs |
| --- | --- | --- |
| Rust release/musl | 94 tests geslaagd (85 terminal,9 onboardingbinary) | evidence/phase4-unit-tests.txt |
| Pine closed-bar | 11 fixtures geslaagd in netwerkloze runtime | evidence/pine-tests.json |
| Risk/OMS/crash/DB/lease-regressie | 37 geïsoleerde controles geslaagd | evidence/phase3-regression.json |
| Studio/Pine/graph/alerts/Parquet/OOS | 26 geïsoleerde controles geslaagd | evidence/phase3-integration.json |
| Inputs/Pine-replay/rapporten/lijn-/botalerts | 14 geïsoleerde controles geslaagd | evidence/final-integration.json |
| Auth/CSRF/sessieverval/logout | 10 controles geslaagd met tijdelijke credentials | evidence/authentication.json |
| Backup/restore | 14 tabellen,1 synthetische testrekening,2 echte Parquetbestanden hersteld; hashes gelijk | evidence/restore.json |
| Cargo/npm advisoryscans | 0 gemelde kwetsbaarheden | evidence/rust-advisories.json, frontend-npm-audit.json, pine-worker-npm-audit.json |
| Trivy runtimeimages, netwerkloos | 0 gemelde kwetsbaarheden in terminal, Pine en PostgreSQL | evidence/*-image.json |
| TypeScript/productiebuild | Geslaagd; Monaco apart lazy loaded | Dockerbuild |
| Browser | Vier echte charts, Pine-preview met AST-inputs, graphvalidatie en bron-/systeemstatus; geen console-errors bij eindcontrole | evidence/phase4-browser.json |

Eerdere images hadden bevindingen. Die zijn verholpen door minimale Alpine/musl-runtimes, verwijdering van ongebruikte npm/yarn/gosu en patching van nghttp2. Oude rapporten blijven onder `*-before-hardening.json`. Een schone advisoryscan is geen volledige veiligheids- of livevrijgave. Docker Scout is niet uitgevoerd na auto-reviewweigering wegens mogelijke metadata-upload; de vervangende Trivy-inspectie had geen netwerk.

Werkelijk gemeten op Apple M1 Pro/Linux arm64: ongeveer153 gevalideerde marktberichten/s over30,4s; validatie/update p95 0,754ms;120 statusrequests met12 gelijktijdig in de testomgeving,0 fouten. Dit meet **geen volledige orderroute of langdurige soak**. Zie [BENCHMARKS.md](docs/BENCHMARKS.md).

## Nog niet af of extern geblokkeerd

- **Fase5:** afzonderlijke Demo-onboardingbinary met native signing, read-only account/fee/snapshotcalls en bounded private kanaalprobe gebouwd. Zonder nieuwe Demo-key nog niet werkelijk uitgevoerd. Continue private reconciliatie, echte fees/cashflows, duurzame exchange-orderadapter, cancel/amend/native bescherming/Cancel All After en effectieve live-fencing zijn **nog niet geïmplementeerd/geaccepteerd**. LIVE blijft technisch uit.
- Echte Jev-calls en vergelijkbare forward-paperproef vereisen nieuwe key, expliciete budgetten en daadwerkelijk verzamelde beslissingen. Geen winstclaim op basis van fixtures/backtests.
- Verder Pine-intrabar/rollback/varip/request.security/strategy.*, vrije docking, chartorder-amendments/snelmodus, business-WS-candles en tick/L2-history blijven open. Volumeprofile/orderflow/heatmaps en DeMARK zijn expliciete datalicentie-/vervolgstappen.
- Geen fysieke disk-fullproef, langdurige soak, onafhankelijke securityreview, echte amd64-run of remote TLS/MFA/VPN-uitrol. Geen cloud/VPS gebouwd.

Paper is EUR spot1×. Fees10bp/fill zijn een model, geen werkelijk accounttarief;500ms minimale submitvertraging en10% deelname aan beste boekliquiditeit. OHLC-replay kent geen echte queue/tickvolgorde. Lokale stops werken niet tijdens Mac-slaap, Dockerstop of ontbrekende prijzen; verlieslimieten zijn actiedrempels. Backups zijn handmatig/maximaal 256 MiB en omvatten geen browser-local tekeningen. Zie handleidingen voor details.

## Tekenbediening vernieuwd · 4 oktober 2026

De negen vormen hebben een zichtbare tekenbalk, tooluitleg, stapsgewijze instructies, live preview en versleepbare vormen/ankers. Kanalen krijgen een derde punt voor breedte; risk/reward ondersteunt een zelfgekozen target en long/short. Tekst, kleur, lock, hide, herhaald tekenen, Esc en undo/redo zijn beschikbaar. Oude v1-tekeningen blijven behouden. Koersupdates herbouwen de grafiek niet meer. Tien gerichte geometrie/opslagtests, TypeScript/productiebuild en interactieve browserproeven slagen. Zie [bediening en testgrenzen](docs/DRAWING_TOOLS.md). Fase5-status en orderbevoegdheden blijven zoals hierboven beschreven.

Deze UI-versie is dezelfde dag lokaal gebouwd en geactiveerd op poort 4780. Container-health: running/healthy. Productiepagina gecontroleerd met de nieuwe tekenbalk en zonder consolefouten.

## Tijdframes uitgebreid · 4 oktober 2026

De grafiek ondersteunt nu 1/3/5/15/30 minuten, 1/2/4/6/12 uur, dag, week, maand en kwartaal. Week en maand zijn snelle knoppen; alle veertien intervallen staan in het gegroepeerde keuzemenu. Backendnormalisatie, indicatoren, actualiteit en tekenankers gebruiken echte kalendergrenzen voor maand/kwartaal. De aparte Pine-/replay-runtime blijft beperkt tot vaste intervallen.

De lokale versie op poort 4780 is bijgewerkt en running/healthy. Alle 100 Rust-tests (91 terminal + 9 onboarding), 11 frontendtekentests en de TypeScript/productiebuild slagen. Alle veertien tijdframes leveren echte actuele BTC-EUR-data zonder gaten; historische week- en maandpaginatie slagen. In de browser zijn week, maand en kwartaal weergegeven en is maandpersistentie na herladen bevestigd. Zie [tijdframes en testgrenzen](docs/TIMEFRAMES.md), `evidence/timeframes-live-2026-10-04.json` en `evidence/timeframes-ui-2026-10-04.jpg`.

## Pine en Bot Studio vereenvoudigd · 5 oktober 2026

Pine-scripts bewaren nu browserconcepten, ondersteunen getypeerde numerieke keuzelijsten en kunnen rechtstreeks als exacte versie naar de terminalchart of een botnode. De chart toont numerieke Pine-plots en entry/exit-reeksen. Bot Studio heeft gewone nodeformulieren, exclusieve if/else-uitgangen, AND/OR/NOT, koop/verkoop/wacht, volledige botinstellingen met Paper-/Live-ontwerpkeuze, versieopslag, kopiëren en import/export. De toegepaste paperbot wordt atomair en gepauzeerd geïnstalleerd. Botbacktests voeren de daadwerkelijke nodegrafiek en botlimieten uit; rapporten blijven bewaard. De terminal toont de bots en hun werkelijk toegepaste versie. Losse Paper-/Backtestpagina’s zijn uit het menu; rekening-/orderbediening is onder Bot Studio bereikbaar.

Verificatie: 109 Rust-tests (100 terminal + 9 onboarding), 11 Pine-fixtures, 11 tekenregressietests, 26 nieuwe geïsoleerde integratiecontroles en de TypeScript/productiebuild slagen. Browserproef: if/else-formulier, kopie en opslag, liveontwerp backtesten, Pine-input 5→10, opslaan naar chart en botweergave; geen consolefouten. Bewijs: `evidence/studio-workflow-rust-tests.txt`, `evidence/studio-workflow-integration.json`.

Live-uitvoering en native Pine `strategy.*` blijven niet beschikbaar. Bots werken expliciet op 1 minuut. Backtests initialiseren historische series op een andere vensterlengte dan forward-paper; stateful scripts kunnen afwijken. Nieuws/Jev worden niet fictief historisch nagebootst. Meer dan twintig plots wordt nu expliciet geweigerd, zodat geen entry/exit-signaal ongemerkt verdwijnt. Zie [bediening en grenzen](docs/BOT_STUDIO.md).

De vernieuwde versie is lokaal geactiveerd op poort 4780 en healthy bevonden. De rekening is nog niet aangemaakt en er zijn nul gebruikersbots gestart. De disposable serverbot heeft een verse if/else-candle werkelijk verwerkt; daarna gepauzeerd. Vier gelijktijdig getoonde Pine-charts zijn in de browser gecontroleerd, zonder consolefouten. Zie `evidence/studio-workflow-server-bot.json` en `evidence/studio-workflow-browser.json`.

## Systeem-instellingen · 5 oktober 2026

Systeem opent met Aansluitingen & AI: gescheiden OKX Demo-/Live-keys (key, secret, passphrase), Jev direct via TypeSafe en Jev via OpenRouter. Sleutels kunnen lokaal versleuteld worden opgeslagen, vervangen en verwijderd. De opgeslagen providerkeuze bepaalt de daadwerkelijke Jev-route. Budgetbediening en kostenoverzicht zijn hier beschikbaar; een provider- of AI-keywijziging via Systeem schakelt AI uit tot herbevestiging. Bestaande systeemmetingen staan onder Status & bescherming.

114 Rust-tests, de productiebuild en 40 geïsoleerde integratiecontroles slagen. Browserbediening, herladen, lege keyvelden na opslaan en lichte/donkere weergave gecontroleerd. Tests gebruikten alleen fictieve credentials in een tijdelijke database. Geen echte provideracceptatie, betaalde calls of orders uitgevoerd. TypeSafe-kosten zijn expliciet berekend uit ontvangen tokengebruik tegen het vastgelegde openbare tarief. Zie [gebruik, opslag en beperkingen](docs/SYSTEM_SETTINGS.md) en `evidence/system-settings-integration.json`.

De Systeem-update is lokaal geactiveerd op poort 4780 en healthy gecontroleerd. Geen gebruikersrekening, providerkeys of kostenbudget ingesteld. Productiebrowser toont de nieuwe instellingen zonder consolefouten. De tijdelijke testdatabase/container en fictieve keys zijn opgeruimd. Bewijs: `evidence/system-settings-local.json` en `evidence/system-settings-ui.jpg`.

## Bot Studio gebruiksgemak · 5 oktober 2026

- Begeleide route Bouwen → Instellingen → Backtest → Start & volg; nodezoeker en uitleg per node.
- Start & volg biedt een expliciet bevestigde virtuele rekening en daarna opslaan/toepassen/starten in één gebruikersactie. Paper heet nadrukkelijk live koersen met virtueel geld; echt-geldstart blijft geblokkeerd.
- Studio en Terminal delen botactiviteit: beslissingen, redenen voor geen order, orders, fills, kosten, open posities en toegepaste versie. De laatste 150 bot-/rekeninggebeurtenissen worden uit duurzame opslag gelezen. Het terminalpaneel gebruikt de volle breedte.
- Hoofdschermen dezelfde beschikbare breedte en buitenmarges; navigatie begint bovenaan. Oude charts ontvangen geen prijs-/Pine-lijnupdates meer tijdens opruimen na een scherm- of themawissel.
- Gecontroleerd: Rust-suite (104 terminaltests + 10 onboardingtests), 26 bestaande workflowcontroles, acht aanvullende uitvoerings-/historiecontroles en browsercontrole van alle hoofdschermen. Een aparte testbot voerde een werkelijke virtuele aankoop op actuele OKX-data uit, meldde bij vervolgsignalen waarom geen tweede order kwam en is vanuit de terminal gepauzeerd. Geschiedenis/pauze overleefden een testserverherstart.
- Bewijs: `evidence/bot-usability-*`. De gebruikersrekening is niet aangemaakt of gebruikt voor tests. Native TradingView strategy.* en echt-geldhandel blijven buiten het huidige uitvoerprofiel.

## Systeembots en candle-stop · 5 oktober 2026

- Vijf ingebouwde voorbeelden beschikbaar via Bot Studio → Systeembots: SMA 9/21, EMA 9/21, RSI 30/70, MACD 12/26/9 en Bollinger terugkeer. Openen maakt een eigen conceptkopie; er wordt niets gestart of toegewezen.
- AI staat standaard uit. Jev-nodes vereisen expliciete opt-in in het document; de backend dwingt dit af. De risicokern blijft deterministisch.
- De instelbare candle-stop toetst opening versus slot van gesloten 1-minuutcandles vóór de normale signaalroute. Drie acties: sluiten en pauzeren, alleen sluiten of alleen pauzeren. Voorbeelden gebruiken een aanpasbare grens van 1% met sluiten/pauzeren.
- Afbouwverzoeken worden duurzaam per bot bewaard, blokkeren nieuwe entries en blijven via de bestaande uitvoerder actief bij pauze of tijdelijk ontbrekende marktdata. Oude rekeningopslag blijft leesbaar. Stopgebeurtenissen en nog lopende afbouw zijn zichtbaar in de activiteit.
- Verificatie: 121 Rust-tests (111 terminal + 10 onboarding), frontendproductiebuild, 32 geïsoleerde integratiecontroles en browserproef slagen. Alle vijf voorbeelden zijn zonder AI-key/kosten op echte historische candles backgetest. Alle drie stopacties zijn door volledige grafiekreplay geverifieerd. Drempelwijziging en pause-only keuze zijn via de UI opgeslagen en teruggelezen.
- De lokale versie is bijgewerkt; bestaande botconfiguraties zijn behouden. Geen systeemvoorbeeld namens de gebruiker gestart. Bewijs: `evidence/system-bots-*`; herhaalbare praktijktest: `scripts/system_bot_checks.py`.


## Indicator 913sig · 5 oktober 2026

De door Willem aangeleverde Pine v5-indicator is omgezet naar het bestaande numerieke profiel, zonder terminalcode te wijzigen. Bron: `pine-scripts/913sig-okx.pine`; opgeslagen als **913sig OKX · v1**, type indicator, eigen paneel. Dalende/stijgende plots bevatten 0 of het oorspronkelijke labelnummer 7/8/9/11/12/13. Setup- en bonusresets blijven gelijk aan het origineel. Pine-labels boven/onder candles en hun afzonderlijke kleuren zijn niet behouden; zie [weergave en gebruik](docs/913sig-okx.md).

Negen gerichte berekeningstests slagen tegen de oorspronkelijke na-/labelvoorwaarden, inclusief 5.000 candles en bonusonderbreking terwijl setup doorloopt. HTTP-preview en de browsereditor slagen op 299 actuele gesloten BTC-EUR-candles van 15 minuten. De exacte opgeslagen bron is teruggelezen en geverifieerd. Bewijs: `evidence/913sig-verification.json`, `evidence/913sig-http-verification.json`, `evidence/913sig-editor.jpg`. Geen entry/exit-plots, handelsvoorstellen, botstart of rekeningwijziging door deze omzetting. Geen brede Pine- of TradingView-pariteitsclaim.


## Native handmatige orderroute · 5 oktober 2026

De Terminal heeft nu een orderticket voor markt, limiet, post-only, IOC, FOK, trigger, TP/SL-OCO, trailing, native smart_iceberg en TWAP. Immutable previews, gelijktijdige reserveringen, duurzame submit/cancel, native ACK/fill/unknown-statussen, afzonderlijke private/business-feeds, fees en native bescherming van filldelen zijn toegevoegd aan een afzonderlijke OKX Demo-gateway. De bestaande Pine-/paperbots blijven paper.

174 Rust-testuitvoeringen (123 terminal, 22 onboarding, 29 gateway), de TypeScript/productiebuild, 32 geïsoleerde HTTP/DB-controles en alle tien orderformulieren in de browser slagen. De TWAP-preview is in de testomgeving zichtbaar gecontroleerd; er is geen native order verzonden. De tests gebruiken synthetische accounts en publieke OKX-data, geen private exchangefixtures die als echte acceptatie worden gepresenteerd. Bewijs: `evidence/native-orders-*`.

De nieuwe versie draait lokaal. De Demo-gateway meldt KEY_REQUIRED, nul native orders. Bestaande gebruikersrekening- en botlimieten zijn vóór/na de update gelijk geverifieerd. Geen key, exchangeaccount, handelsbudget of Livevrijgave namens Willem ingevuld.

**Echt-geldhandel werkt nog niet.** Keys en persoonlijke account-/instrument-/budget-/limietbevestiging ontbreken; echte Demo-acceptatie is niet uitgevoerd. Live heeft een technische blokkade. Dit is EUR spot in basishoeveelheid, zonder volledige OKX-pariteit. Amend, derivaten, gezamenlijke paper/native/bot-ledger en volledige cashflow/afbouwacceptatie ontbreken nog als implementatie, niet uitsluitend als key. Zie [gebruik en exacte grenzen](docs/ORDERS.md).
