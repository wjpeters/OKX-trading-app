# Training Lab · lokale onderzoeksversie · 6 oktober 2026

Open de terminal op `http://127.0.0.1:4780` en kies **Bot Studio → Training Lab**. De nieuwe route is onderzoek met virtueel kapitaal. De volledige aangeleverde opdracht staat in [TRAINING_LAB_SPEC.md](TRAINING_LAB_SPEC.md).

## Bediening en visuals

- **Instellen:** gesloten historie op UTC-datums importeren. BTC-EUR en ETH-EUR zijn het eerste profiel. Onvolledige coverage of ontbrekende candles blokkeert training.
- **Trainen:** CMA-ES, generatie, seeds, rekentijd en netwerk bekijken. Sleep het netwerk om te draaien. Klik op een neuron voor zijn verbindingen. Positieve gewichten zijn cyaan, negatieve roze. De verborgen laag is violet. **Gewichten** toont de echte matrices. Zonder kandidaat is dit expliciet een architectuurpreview.
- **Netwerkcheckpoint:** de gewichten van een eerdere generatie terugkijken. Een historische snapshot heeft geen verzonnen actuele activaties.
- **Vergelijken:** dezelfde kosten en virtuele allocatie voor trend, breakout, mean reversion, zonder Laya, cash en buy-and-hold. Buy-and-hold gebruikt dezelfde maximaal 10% onderzoeksallocatie, niet 100% van het virtuele vermogen.
- **Kwalificeren:** de complete vooraf bepaalde campagnefamilie eenmalig laten beoordelen. Alle overlappende holdoutdata voor dezelfde markt worden daarna als bekeken behandeld, ook bij afwijzing of technische fouten. Holm-correctie wordt pas definitief na alle familieresultaten.
- **Volgen:** alleen historisch geslaagde exacte versies mogen naar forward-paper. Deze bewijsadministratie staat los van je bestaande gebruikersrekening. Pauze onderbreekt nieuwe entries, terwijl exits op beschikbare candles doorgaan.
- **Vrijgeven:** toont de ontbrekende stappen. De persoonlijke native vrijgave bindt één exact artifact aan rekening, omgeving, markt, timeframe, budget en alle limieten. De bevestiging blijft geblokkeerd zonder werkelijk historisch, forward- en voor Live ook Demo-bewijs. De passende gatewayversie is op 6 oktober om 14:39 geïnstalleerd na expliciete toestemming.

Beide twaalfmaandsdatasets zijn op 6 oktober volledig opgehaald: per markt 525.600 gesloten candles, coverage 100% en geen gaten. De eerste campagne is gestart met het berekenen van Laya-marktregimes. De lokale `training-profile.json` bevat één hervatbaar startverzoek voor BTC/ETH op 1m/5m/15m. Zodra beide datasets volledig en Laya gereed zijn, begint de trainer. Ontbrekende historie leidt tot een zichtbare afwijzing. Het startprofiel gebruikt expliciete historische **aannames**: 0,10% takerfee per zijde, 10 bps spread en 5 bps slippage. Dit zijn geen actuele accountfees en geen bewijs van historische fees. Het wijzigen van die aannames vraagt een nieuwe campagne; bestaande campagnes blijven onveranderlijk.

Bij de eerdere controle stond de campagne gepauzeerd na 51.216 Laya-featureberekeningen. Bij de gatewaycontrole om 14:39 draait zij weer met 72.816 featureberekeningen; de agent heeft haar niet hervat. Trainer en Laya zijn online. Dit is een momentopname; er is nog geen getrainde eindkandidaat.

## Netwerk exporteren en importeren

In **Bot Studio → Training Lab** staat **Netwerkarchief**:

1. Kies een vastgezette kandidaat, **Laatste tussentijdse netwerk**, of een eerdere generatie bij **Netwerkcheckpoint**.
2. Klik **Netwerk exporteren**. Het JSON-bestand bevat alle 340 gewichten en biases, strategieparameters, normalisatie inclusief trainingsperiode, markt/timeframe, datasetversie, seed/generatie, featureversie en Laya-checkpoint/rekenversie.
3. Klik **Netwerk importeren** en kies zo'n bestand. Het netwerk wordt als onderzoekskopie in de lokale database opgeslagen en direct getoond. **Bewaard netwerk** laadt eerdere imports terug. Je kunt ze opnieuw exporteren en de matrices bekijken.
4. Kies **Campagne / huidig netwerk** om terug te gaan naar de campagne. Import wijzigt of pauzeert haar niet.

Export wacht niet op afronding van de campagne. De lopende beste versie wordt uit de atomair opgeslagen voortgang gelezen; checkpoints uit een volledige berekende generatie. Vóór de eerste generatie bestaan nog geen getrainde gewichten en blijft export uitgeschakeld. Laya-featurevoorbereiding levert op zichzelf nog geen getraind netwerk op.

Bestandsformaat: `okx-neuro-network`, versie 1. SHA-256 controleert de canonieke netwerkvelden, niet de identiteit van de auteur. Gewichten en normalisatie behouden hun exacte floating-pointwaarde bij JSON-/database-roundtrip. Onjuiste dimensies, parameters, normalisatie, modelversies, gewijzigde hashes en onbekende velden worden afgewezen. Maximaal 128.000 bytes per upload. Een herhaalde import van hetzelfde netwerk gebruikt de bestaande opgeslagen versie.

Het bestand bewaart het **netwerk**, niet de volledige CMA-optimizer, random state, populatie, datasetbestanden of Laya-cache. De oorspronkelijke campagne kan vanaf haar bestaande lokale optimizercheckpoint blijven hervatten. Een geïmporteerd netwerk starten als nieuwe trainingsseed of als nieuwe handelsbot is een afzonderlijke stap en wordt hiermee niet automatisch gedaan.

Testresultaten, historische/forward-kwalificatie, rekeninglimieten, sleutels en Demo-/Live-vrijgave worden niet geïmporteerd. De archiefkopieën staan in een afzonderlijke onveranderlijke tabel; import maakt geen campagnekandidaat of uitvoerbare vrijgave. Het normale onderzoeksrapport is een ander bestand dan de draagbare netwerkexport.

API: `GET /api/training/artifacts/{hash}/export`, `GET /api/training/campaigns/{id}/export?checkpoint={naam}`, `GET /api/training/networks`, `GET /api/training/networks/{id}` en `POST /api/training/networks/import` met `{name,bundle}`. De import heeft dezelfde login-/Originbescherming als de rest van de terminal en een afzonderlijke groottebegrenzing.

## Wat is gebouwd

De Rust-kern `neuro.rs` is gedeeld door de offline evaluator, forward-paper en native neurobotbeslissingen. Zij bevat causale features, training-only normalisatie, de 16 → 16 → 4 netwerkberekening, drie vaste strategieën, vaste exitregels, Decimal-kosten en conservatieve OHLC-fills. Het netwerk heeft 340 gewichten en biases. Het leert geen rekeningbudgetten of harde limieten. Partial fills worden door volume-participatie begrensd; entries zijn op de volgende candle. Latency wordt als ongunstige beweging binnen die candle gemodelleerd. Stops hebben voorrang bij een candle die zowel stop als target raakt. Basefees en rebates worden in de virtuele hoeveelheid/opbrengst verwerkt. Andere roundtripkosten schalen mee in de 1,5× kostenstress.

Python gebruikt pycma 4.4.4 en NumPy 2.3.3: 64 kandidaten, seeds 17/41/73, maximaal 200 generaties per seed/ronde, patience 30, twee evaluators. Vier onafhankelijke 180/30-dagenrondes selecteren de seedconfiguratie; refit gebruikt uitsluitend de ontwikkelperiode. De laatste 60 dagen blijven tijdens zoeken en refit ongezien. Checkpoints bewaren optimizer en random state. Na uitputting wacht de trainer op nieuwe data. De daglimiet is maximaal acht uur in Europe/Amsterdam en blijft na een herstart bewaard.

Laya 0.3.28 gebruikt het vastgezette English-checkpoint `7b928d828b7b0e022f929d9bd2e44165aa270148`. Alle vijf model-/config-/tokenizerbestanden worden op vooraf vastgelegde SHA-256 gecontroleerd. Torch 2.8 draait lokaal eager FP32 op Apple MPS waar beschikbaar. Het werkelijke rekenprofiel wordt in de campagne en het artifact vastgezet. Marktkenmerken worden als een compact causaal verzoek aangeboden en gecachet; er zijn geen betaalde Jev-calls. De service bindt aan loopback. De trainer krijgt geen exchangecredentials of orderroute.

PostgreSQL bevat campagnes, immutable artifacts/configuraties, kwalificatieresultaten, forwardstate en audit. Parquet behoudt exacte decimaalteksten en manifests bevatten bron, hash, datums, coverage en gaps. Publieke business-WebSocket-candles worden opgeslagen met REST-herstel; trades en boekwaarnemingen worden bemonsterd voor latere replay. Die tape is nog geen volledig queue-/maker-fillmodel.

Resourceprofiel: trainercontainer inclusief twee Rust-kindprocessen maximaal 1 GiB en één CPU; terminal maximaal 512 MiB. Laya heeft een harde MPS-allocatiegrens van 2,3 GiB en een monitor van RSS plus drivergeheugen, maximaal 2,5 GiB. Die monitor controleert na modelgereedheid elke 500 ms en stopt de modelservice bij overschrijding. Dit is geen macOS-kernelgrens voor totale RSS: een korte overshoot tussen samples is niet uitgesloten. Datasetassemblage loopt één voor één. De modelstartpiek, een volledige twaalfmaandscampagne en haar piekgeheugen zijn nog niet geaccepteerd.

## Acceptatie en resterend werk

248 Rust-tests, zes Python-trainertests, twintig onderzoekscontroles en achttien native botcontroles slagen. De integratieproef gebruikt een geregistreerde wegwerpdatabase en kunstmatige trainingsdata; een aparte import leest echte openbare BTC-EUR-candles. De onderzoeksproef initialiseert geen gebruikersrekening en plaatst geen orders. De native proef gebruikt uitsluitend kunstmatige rekening-, kwalificatie- en candlefixtures in dezelfde wegwerpdatabase. Zij schrijft een begrensd voorstel in de database, zonder private gateway, exchangeaanvraag of fictieve fill. Een echte korte CMA-proef gebruikt twee Rust-evaluators, drie seeds en zes generaties. Hervatten levert dezelfde kandidaten op. Korte resultaten worden afgewezen als onvoldoende statistisch bewijs. Het visuele testbeeld is duidelijk gemarkeerd als kunstmatige data en is geen gekwalificeerde bot.

De gemeten strategiestap onder een synthetische CMA-belasting heeft p95 203 µs. De eerste Laya-aanvraag in die proef was 5,8 s; daarop is de warm-up uitgebreid. Na de uitgebreide warm-up bleven zes HTTP-inferenceaanvragen tijdens echte featurevoorbereiding onder 365 ms. Zie ook [warm-upbewijs](../evidence/training-laya-warm.json). Deze kleine steekproef en de strategiemeting zijn geen acceptatie van candle → duurzaam native ordervoorstel onder twee seconden. Zie [latencybewijs](../evidence/training-latency.json), [trainingscontroles](../evidence/training-integration.json) en [Rust-bewijs](../evidence/training-rust-tests.txt).

**De volledige opdracht is nog niet geaccepteerd.** Open onderdelen:

1. Echte OKX Demo-acceptatie met partial fills, native bescherming, onbekende orderstatus, reconciliatie, herstart en fencing. De bestaande accountkoppeling gaf vóór deze uitbreiding HTTP 401 / OKX 50110 wegens IP-toelating. Geen accountrechten of IP-instellingen gewijzigd.
2. Geslaagde historische kwalificatie, vervolgens werkelijk ten minste 30 kalenderdagen én 100 forward-roundtrips. Zowel verstreken kalenderduur als ontvangen gesloten candles moeten de duur aantonen. Er is nog geen geschikte bot vastgesteld; die bewijsduur kan niet worden overgeslagen.
3. Volledige duurproef, geheugenpieken, modelconcurrentie en het latencydoel voor duurzame native voorstellen. De kleine fixtureproef onder twee seconden is geen p95-acceptatie onder volledige belasting. Sampling van boeken/trades uitbreiden voordat maker-uitvoering wordt onderzocht.
4. Het tijdvenster tussen bevestigde annulering van native OCO-bescherming en de begrensde IOC bij maximale houdduur op echte Demo bewijzen. Partial fills, timeout, herstart en restinventaris blijven deel van die acceptatie.

Beide gateways zijn op 6 oktober om 14:39 geïnstalleerd na Willems expliciete toestemming. De eerder geweigerde installatie is daarmee uitgevoerd. De codehash en accountbehoud zijn gecontroleerd: [installatiebewijs](../evidence/training-gateway-update.json). Dit vervangt geen echte Demo-/forwardacceptatie.

## Gebouwde native keten en grenzen

`native_bots.rs`, `native_bot_risk.rs` en `gateway_neuro.rs` gebruiken dezelfde causale feature- en strategiesignalen als de evaluator. De persoonlijke vrijgave slaat artifact, account-UID, omgeving, instrument/timeframe, budget, limieten, policyrevision en uitvoeringscodehash onveranderlijk op. De codehash omvat strategie, risico- en gatewaybron. Een codewijziging vraagt een passende gateway en nieuwe expliciete vrijgave. De historische kostenstress begrenst ook actuele fee, spread en IOC-prijsmarge.

Elke bot heeft eigen werkelijke inventaris, netto fees, cashadministratie en verliesbasis. Rekeningbrede transactielocks/reserveringen tellen handmatige orders en alle bots mee. Botlimieten mogen rekeninglimieten niet verruimen; drawdown is maximaal 5% van het botbudget. Per entry geldt maximaal 10% van dit budget en de persoonlijke orderlimiet. Maximaal acht actieve bots per rekening en twee gelijktijdige besluitberekeningen zijn begrensd. De worker scant maximaal honderd recentste native runs; zeer grote historische runverzamelingen zijn nog geen bewezen schaalprofiel.

Een botvoorstel is uniek per gesloten candle. De proposal-latency loopt van de eerste duurzame candleontvangst tot de gecommitteerde queue-transactie, zonder exchange-ACK. De gateway hercontroleert actuele rekening, release, code, risico, kosten en marktregels voor daadwerkelijke dispatch. Ontbrekende/verouderde data, Laya-fouten en ontbrekende bescherming blokkeren nieuwe entries. Pauze of revoke annuleert ongestuurde entries; bestaande bescherming en reduce-only uitwegen blijven actief.

De gateway bewaakt vaste maximale houdduur vanaf de eerste werkelijke fill, onafhankelijk van model en nieuwe candles. Hij annuleert eerst eventueel resterende entry en native OCO, reconcilieert bevestigde fills, en verkoopt uitsluitend de werkelijk resterende eigen inventaris via een begrensde IOC. UNKNOWN wordt opgevraagd, nooit blind opnieuw verzonden. Partial exits blijven gevolgd; onder-minimumrest wordt als dust bijgehouden. De bestaande stop wordt niet verruimd. Een reducing exit kan na revoke of gewijzigde entrypolicy blijven werken, maar mag geen inventaris van een andere bot gebruiken.

Echte Demo-acceptatie verlangt voor dezelfde artifact-/codeversie werkelijke entry- en sellfills, partial fills, native bescherming, herstel van UNKNOWN en meerdere gatewaystarts na release. Alleen een queued fixturevoorstel kan die acceptatie niet behalen. De oude compatibiliteitsroute `/api/training/artifacts/{hash}/release` blijft geblokkeerd; het persoonlijke formulier gebruikt `/native-release`. Pauze, hervatten, revoke en Demo-bewijs verlopen via `/api/training/native/{id}/...`.

Bewijs van de geïsoleerde native proef: [achttien controles en fixturelatency](../evidence/training-native-integration.json). Bestaande accountpolicy, persoonlijke Live-vrijgave en orderaantallen zijn lezend vergeleken: [behoudcontrole](../evidence/training-preservation.json). Geen neurobot is op Willems rekening vrijgegeven.

## Starten, stoppen en bewijs

**Start OKX Terminal.command** start terminal/trainer en lokale Laya. De wachtende onderzoeksstart wordt hervat. **Stop OKX Terminal.command** stopt ook de profielstarter en Laya. Browser sluiten stopt de trainer niet. Mac-slaap, Dockerstop en netwerkuitval onderbreken de lokale uitvoering.

```sh
rtk proxy .local/laya-venv/bin/python scripts/training_checks.py pipeline
rtk proxy .local/laya-venv/bin/python scripts/training_checks.py provision
rtk proxy .local/laya-venv/bin/python scripts/training_checks.py check
rtk proxy .local/laya-venv/bin/python scripts/training_checks.py cleanup
rtk proxy docker run --rm --network none --entrypoint python okx-training-lab:0.1.0 -m unittest -v test_worker
```

`pipeline` vraagt pyarrow als testafhankelijkheid. Productietraining leest Parquet via Rust en heeft pyarrow niet nodig. Alleen de opgenomen testcontainer en testdatabase mogen worden opgeruimd. De bestaande Git-wijzigingen van andere OKX-werkzaamheden zijn behouden; er is geen algemene commit of reset uitgevoerd.

Bronnen voor de gekozen implementaties: [Laya](https://github.com/NandhaKishorM/laya), [pycma](https://github.com/CMA-ES/pycma), [OKX-fee-API](https://app.okx.com/docs-v5/en/#trading-account-rest-api-get-fee-rates). Actuele gateway-fees blijven een afzonderlijke accountbron; historische aannames worden niet daarvan afgeleid.
