# Bouwstatus · 2 oktober 2026 · versie 0.4.0

**De lokale paper-/onderzoeksapp is sterk uitgebreid met fase3/4-functies. De volledige bouwprompt is nog niet afgerond: fase5-exchange-uitvoering en enkele geavanceerde onderdelen ontbreken.** De volledige scope blijft staan in BUILD_SPEC.md; [REMAINING.md](docs/REMAINING.md) maakt ontbrekende implementatie en ontbrekende acceptatie afzonderlijk zichtbaar.

Open http://127.0.0.1:4780 via **Start OKX Terminal.command**. Alles draait lokaal, buiten iCloud. De gebruikersrekening is nog niet aangemaakt, nul gebruikersbots actief, geen betaalde Jev-call of exchangeorder uitgevoerd. Budgetten, keys en livevrijgave zijn niet namens Willem gekozen.

## Werkend en getest

- **Marktterminal:** twaalf echte EUR/USDC-spotmarkten, `books` snapshot/delta met sequencecontrole, tape/tickers, zes timeframes, tien instelbare native indicatoren, één tot vier gekoppelde charts, tijdzones, charttypen, fullscreen, command palette, workspaces en links/rechts te plaatsen panelen.
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
