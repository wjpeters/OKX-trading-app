# Bouwprompt — privé OKX tradingterminal

Documentatie gecontroleerd op 2 oktober 2026. Dit is een productspecificatie en implementatieopdracht, geen bewijs van een werkende integratie, winstgevendheid of behaalde latency. Controleer tijdens implementatie opnieuw de actuele documentatie, licenties en accountrechten. De bronnen onderaan horen bij deze prompt.

## 1. Rol, doel en uitgangspunten

Je bent principal software architect, senior trading-systems engineer, security engineer en productdesigner. Ontwerp en bouw een professionele, single-user crypto-tradingterminal voor mijn privégebruik op OKX. Lever werkende software, geen dashboard met fictieve functionaliteit.

De terminal moet TradingView-achtige analyse combineren met rechtstreeks handelen, een Pine-compatibele editor/runtime, nieuws, sentiment, backtesting en visueel programmeerbare tradingbots. Bots mogen Jev van TypeSafe gebruiken. Alles draait in Docker: lokaal voor ontwikkeling en paper trading, op mijn VPS voor 24/7-uitvoering. Een gesloten browser mag bots, alerts en risicobewaking niet stoppen. Self-hosting betreft de applicatie; OKX en Jev via OpenRouter blijven externe diensten. Veronderstel geen lokaal beschikbare Jev-modelgewichten.

Prioriteit: correcte orderuitvoering en bescherming van kapitaal, daarna meetbare snelheid, vervolgens uitbreidbaarheid en visuele kwaliteit. Beloof geen winst, absolute verliesgarantie, volledige Pine-compatibiliteit of institutionele HFT-latency zonder bewijs. Begin met spot zonder leverage; ontwerp uitbreidingen voor andere producten alleen voor zover mijn account deze toestaat.

## 2. Architectuur en technische keuzes

Gebruik actuele stabiele versies en leg exacte versies vast in lockfiles, container-images en architectuurbesluiten. Gebruik geen ongepinde `latest`-images in productie.

Voorkeursstack: React, TypeScript strict en Vite voor de frontend; Tailwind CSS en shadcn/ui voor het designsysteem; TanStack Query/Table en een kleine state-store voor UI-state; Lightweight Charts voor charts; Monaco voor code; React Flow voor de boteditor. Bouw ontbrekende analyse-, teken- en tradingfunctionaliteit zelf boven op de chartbasis.

Gebruik Rust met Tokio voor marktdata, incrementele berekeningen, risicocontroles, ordermanagement en replay. Gebruik Axum voor de backend-API en frontend-WebSocket. Zet Pine-uitvoering en eventueel een TypeScript/Jev-adapter in geïsoleerde workers. Houd de uitvoeringskern klein: geen microservice per indicator.

Gebruik PostgreSQL voor configuraties, orders, fills, risicostatus en auditregistratie. Gebruik gecomprimeerde Parquet-bestanden voor omvangrijke historische datasets. Voeg TimescaleDB, ClickHouse, Redis of een message broker uitsluitend toe na een aantoonbare behoefte en een onderbouwde keuze; maak ze niet allemaal verplicht.

Scheid de snelle tradingroute van de besturingslaag voor UI, nieuws, backfills en rapportages. Browser, AI en scripts krijgen nooit exchange-credentials. Het centrale execution-gateway is de enige component die orders mag versturen. Documenteer procesgrenzen, datacontracten, eigenaarschap en herstelgedrag.

## 3. Professionele terminal en bediening

Ontwerp een premium desktop-first terminal met rustige donkere standaardweergave, optionele lichte modus, scherpe typografie, tabulaire cijfers, hoge informatiedichtheid en duidelijke visuele hiërarchie. Geen decoratieve neon-effecten, onnodige animaties of grote lege dashboardkaarten. Maak tekstlabels naast statuskleuren verplicht.

Gebruik een configureerbare indeling: navigatie links; symbolen, timeframe en workspace bovenaan; één tot vier gekoppelde charts centraal; orderboek, transactiestroom en orderticket rechts; posities, orders, fills, alerts en botlogs onderaan. Ondersteun versleepbare panelen, opgeslagen workspaces, fullscreen, toetsenbordbediening en command palette.

Maak aparte werkruimtes voor Terminal, Screener, Pine Editor, Bot Studio, Backtests, Nieuws/Sentiment en Systeemstatus. Toon permanent de uitvoeringsomgeving, account, actieve bots, dataversheid, verbindingen, risicostatus en gemeten latency.

Chart-trading moet orderlijnen, positielijnen, stop-loss/take-profit en een risk/reward-preview ondersteunen. Een live-order vereist een duidelijke preview en bevestiging, tenzij ik expliciet een beveiligde snelhandelsmodus activeer. Handmatige orders volgen dezelfde centrale risicocontroles als bots.

Laat loading, ontbrekende data, vertraging, fouten en restricted features eerlijk zien. Verzin nooit prijzen, nieuws, fills of resultaten om het scherm te vullen.

## 4. OKX-koppeling en accountdetectie

Gebruik de native OKX V5 REST- en WebSocket-API's. Neem niet automatisch aan dat globale API-hosts, USDT-paren, derivaten of VIP-kanalen voor mijn Nederlandse/EEA-account beschikbaar zijn. Controleer accountregio, instrumenten, handelsrechten en datatoegang tijdens onboarding. Omzeil geen regionale beperkingen.

Gebruik voor een bevestigd EEA-account deze configureerbare uitgangspunten, met poort 443:

```text
REST live/demo:  https://eea.okx.com
WS live public:  wss://wseea.okx.com/ws/v5/public
WS live private: wss://wseea.okx.com/ws/v5/private
WS live business:wss://wseea.okx.com/ws/v5/business
WS demo public:  wss://wseeapap.okx.com/ws/v5/public
WS demo private: wss://wseeapap.okx.com/ws/v5/private
WS demo business:wss://wseeapap.okx.com/ws/v5/business
```

Gebruik geen `:8443`: OKX heeft aangekondigd deze poort op 31 oktober 2026 te sluiten. Demo gebruikt eigen keys en waar vereist `x-simulated-trading: 1`.

Integreer ten minste deze API-groepen en verifieer parameters in de actuele regionale documentatie:

```text
GET  /api/v5/public/time
GET  /api/v5/public/instruments
GET  /api/v5/account/instruments
GET  /api/v5/market/candles
GET  /api/v5/market/history-candles
GET  /api/v5/market/history-trades
GET  /api/v5/account/trade-fee
GET  /api/v5/account/balance
GET  /api/v5/account/positions
POST /api/v5/trade/order
POST /api/v5/trade/cancel-order
POST /api/v5/trade/amend-order
GET  /api/v5/trade/orders-pending
GET  /api/v5/trade/fills
POST /api/v5/trade/order-algo
POST /api/v5/trade/cancel-all-after
```

Gebruik passende public/business-kanalen voor tickers, trades, candles en orderboeken; private-kanalen voor orders, balances en posities. Selecteer kanalen op werkelijk beschikbare rechten. Implementeer een capability-matrix en documenteer rate limits per endpoint, kanaal, account en instrument. Reserveer capaciteit voor risicoreducerende acties.

## 5. Marktdata, historie en datakwaliteit

Maak OKX de gezaghebbende prijsbron voor uitvoeringsbeslissingen. Haal historie gepagineerd op, ondersteun hervatbare imports en onderzoek OKX Historical Market Data voor grotere trade- en L2-datasets. Rapporteer dekking per instrument, resolutie en datum; veronderstel geen onbeperkte tickhistorie.

Beheer per orderboekkanaal de juiste snapshot/delta-semantiek. Controleer continuïteit met `seqId/prevSeqId` volgens de actuele regels, inclusief heartbeat- en resetgevallen. Bouw geen integriteitscontrole op het vervallen checksumveld. Combineer geen REST-snapshot en WS-delta's zonder bewezen compatibele sequence-aansluiting.

Implementeer reconnect, jitter, backoff, resubscribe, gap-detectie, resync, deduplicatie en begrensde buffers. Bij datagaten: markeer het instrument onbetrouwbaar, blokkeer nieuwe blootstelling en herstel eerst de toestand. Bestaande bescherming blijft actief.

Bewaar exchange-tijd, ontvangsttijd, verwerkingstijd en datakwaliteit. Gebruik UTC voor opslag en een monotone klok voor duurmetingen; laat de gebruiker tijdzones kiezen. Verwerk open en gesloten candles expliciet en bereken hogere timeframes consistent. Vul ontbrekende marktdata nooit ongemerkt met verzonnen waarden.

Gebruik decimal/fixed-point voor geld, prijsstappen, aantallen en fees. Valideer tick size, lot size, minimumorder, contracteenheid en toegestane afrekenvaluta. Bereken indicators incrementeel, niet op iedere tick over de complete historie.

## 6. Charts, indicators, tekenen en alerts

Ondersteun candles, OHLC, lijnen, volume, Heikin-Ashi, meerdere schalen, gekoppelde crosshairs en meerdere timeframes. Houd synthetische chartprijzen strikt gescheiden van verhandelbare prijzen.

Lever eerst SMA/EMA, RSI, MACD, ATR, Bollinger Bands, VWAP, Stochastic, ADX en OBV, met configureerbare inputs, eigen panelen, stijlopties en unit tests. Plan volume profile, orderflow en heatmaps als expliciete vervolgstappen waarvoor de benodigde data werkelijk beschikbaar moet zijn.

Neem mijn wens voor Tom DeMark/Sequential/Combo mee. Controleer vooraf rechten, licenties en toegestane specificaties. Gebruik uitsluitend een toegestane implementatie of een duidelijk benoemde, zelfstandig gespecificeerde variant. Een eenvoudige 9/13-teller is niet automatisch een volledige officiële indicator. Documenteer regels, varianten, bron en afwijkingen; test tegen rechtmatig beschikbare referentiegevallen.

Bouw trend-, horizontale en verticale lijnen, kanalen, Fibonacci, rechthoeken, tekst, meettools en risk/reward-tools. Ondersteun magnet snapping, selecteren, undo/redo, vergrendelen en persistente tekeningen.

Alerts moeten server-side blijven draaien en prijs, indicator, tekening, bot- en risicocondities ondersteunen. Maak candle-close versus intrabar expliciet; voeg cooldown, deduplicatie en afleverstatus toe.

## 7. Eigen Pine-compatibiliteitslaag en editor

Bouw een eigen Pine-compatibiliteitslaag met editor, parser/runtime-adapter, tests en documentatie. Doel is het importeren en draaien van echte toegestane `.pine`-scripts, niet alleen JavaScript dat op Pine lijkt.

Onderzoek PineTS als mogelijke runtimebasis, maar beoordeel de actuele AGPL/commerciële licentie, dekking, onderhoud, beveiliging en snelheid. Houd de runtime vervangbaar. Wanneer zelfbouw nodig is: gebruik een echte parser, AST, typecontrole en evaluator; geen regex-transpiler die oppervlakkig enkele voorbeelden accepteert.

Richt je op Pine v5/v6 via gefaseerde, aantoonbare compatibiliteit. Leg ondersteunde syntax, functies en semantiek vast in `PINE_COMPATIBILITY.md`. Ondersteun stapsgewijs series, history indexing, inputs, `na`, persistente variabelen, TA-functies, plots, alerts, control flow, collecties, meerdere timeframes en strategy-functies.

Test vooral historische versus realtime-uitvoering, rollback, `var`/`varip`, barstatus, intrabar-herberekening en `request.security()` met gaps/lookahead. Onbekende functies moeten een concrete compilefout opleveren, nooit stilzwijgend een ander resultaat. Signaleer repainting en toekomstlekken.

De editor krijgt highlighting, autocomplete, diagnostiek met regelnummers, versiebeheer, import/export, parameterformulieren en preview. Chart, backtest, paper en live gebruiken dezelfde gevalideerde berekeningssemantiek. Strategy-calls produceren uitsluitend order-intents voor de centrale risk engine.

Voer scripts uit in geïsoleerde processen/containers met CPU-, geheugen-, looptijd- en uitvoerlimieten, zonder credentials, hostbestanden of onbeperkt netwerk. Alleen een JavaScript worker of `eval` is geen beveiligingsgrens. Omzeil geen protected of invite-only scripts.

## 8. Bot Studio

Maak een visuele node-editor én een code/configuratieweergave met één canonieke, versieerbare strategiespecificatie. Compileer deze naar een gevalideerd uitvoeringsplan; de UI-grafiek zelf is niet de trading-engine.

Nodes omvatten marktdata, indicators/Pine, voorwaarden, logische combinaties, tijdvensters, cooldowns, nieuwsfeatures, Jev-beslissingen, position sizing en order-intents. Verwerp onbegrensde lussen en ongeldige verbindingen. De centrale risk gate is verplicht en kan niet met een node worden verwijderd.

Per bot configureer ik naam, instrumenten, strategieversie, budget, modus, triggers, maximale frequentie, Jev-gebruik en risicolimieten. Toon status, equity, exposure, drawdown, fees, AI-kosten, beslissingen en afwijsredenen. Ondersteun pauzeren, stoppen, klonen, rollback en gecontroleerd promoveren naar live.

Lever educatieve templates voor trend-following, range/mean-reversion en een Jev-regimefilter. Geen winstclaims; martingale, onbeperkt middelen en leverage staan standaard uit. Wijzig nooit ongemerkt een draaiende live-strategie.

## 9. Jev-integratie

Gebruik OpenRouter als standaardprovider, passend bij mijn bestaande gebruik. Ontwerp een providerinterface die later ook direct TypeSafe kan ondersteunen. Bouw geen automatische provider- of modelwissel in live-trading.

De gecontroleerde uitgangspunten zijn model `typesafe/jev-1.13`, OpenRouter Decisions API `POST https://openrouter.ai/api/alpha/decisions`, of de gedocumenteerde System One-route voor de TypeSafe SDK. Controleer het actuele schema; behandel Jev niet als gewone chatcompletion met vrije JSON-tekst. Pin een geteste modelversie, niet de bewegende latest-alias.

Bereken cijfers, indicatorwaarden, fees, tijdsvergelijkingen, sizing en risicolimieten deterministisch. Geef Jev compacte, relevante state en expliciete Choice/Noul/Score-vragen, bijvoorbeeld marktregime, relevantie van nieuws of keuze uit vooraf toegestane acties.

Gebruik intern een eigen DecisionEnvelope met decision-id, snapshot-id, strategieversie, model-id, antwoord, werkelijk ontvangen probabilities/confidence, ontvangsttijd, vervaltijd en kosten. Dit is een applicatieschema, niet een verzonnen providerresponse. Modelconfidence is geen bewezen winstkans.

Roep Jev alleen aan op relevante events en na een eerste risicofilter, niet blind op iedere tick. Begrens gelijktijdigheid, kosten en wachttijd. Verwerp late antwoorden wanneer markt- of accountstate gewijzigd is. Herhaal de risicocontrole direct vóór verzending van een order.

Bij timeout, ongeldige output, lage zekerheid of circuit-breaker: geen nieuwe blootstelling voor een AI-afhankelijke bot. Stops en andere noodzakelijke beschermingsacties blijven deterministisch functioneren. Bewaar beslissingsinputs voor replay, maar geen secrets. Verzin geen redeneertekst die het model niet retourneert.

## 10. Risicobeheer en verlieslimieten

Maak limieten verplicht per bot én account: maximaal toegewezen kapitaal, risico per transactie, verlies per dag en cumulatief, drawdown, positiegrootte, open posities, open orders, leverage, spread, slippage, data-ouderdom, handelsfrequentie, opeenvolgende verliezen en AI-budget. Toon bedragen én percentages met expliciete grondslag.

Definieer dagverlies en drawdown exact. Neem gerealiseerde én ongerealiseerde P&L, fees, funding, reserves en cashflows mee. Gebruik conservatieve waardering en persistente dagstart/high-water-mark. Een herstart mag de verlieslimiet niet resetten. Leg de gekozen daggrens en tijdzone vast. Bepaal een aparte limiet voor de totale equitydaling door alle bots samen. Livegang zonder expliciet ingevulde en bevestigde limieten is niet toegestaan.

Reserveer budget atomair voor open orders en partial fills. Voorkom dat gelijktijdige bots hetzelfde geld dubbel gebruiken. Controleer totale blootstelling aan dezelfde onderliggende munt. Gebruik waar beschikbaar aparte subaccounts; zonder echte scheiding zijn virtuele botbudgetten geen gegarandeerde kapitaalisolatie. Verwerk ook handmatige en extern geplaatste orders in accountbrede controles.

Onderscheid `PAUSE_ENTRIES`, `CANCEL_ENTRY_ORDERS` en `EMERGENCY_FLATTEN`. Definieer per storing welke toestand geldt. Annuleer geen beschermende orders voordat de positie bevestigd gesloten of vervangend beschermd is. Gebruik een expliciet beleid voor afbouw bij slechte liquiditeit: een slippagelimiet en gegarandeerd onmiddellijk sluiten zijn niet tegelijk afdwingbaar.

Verlieslimieten zijn actiedrempels, geen absolute financiële garantie bij gaps, slippage, onbereikbaarheid of exchangeproblemen. Toon dit in de UI. Hervatten na een risicostop vereist expliciete controle; een bot of AI mag de limiet niet zelf verhogen.

## 11. Ordermanagement en herstel

Implementeer een expliciete order-state-machine voor intent, pending submit, accepted, partial fill, filled, pending cancel, canceled, rejected en unknown. Een acknowledgement is geen fill. Houd instrument-specifieke ordertypes en aantalsemantiek correct bij.

Leg intent en kapitaalreservering duurzaam vast vóór verzending. Gebruik unieke client-order-id's, deduplicatie en correlatie-id's. Houd latency van deze noodzakelijke duurzaamheid zichtbaar; verberg deze niet uit end-to-end-metingen.

Een timeout na orderverzending betekent een onbekende status, niet automatisch een mislukte order. Controleer exchange-status en reconcile vóór iedere mogelijke herverzending. Claim geen magische exactly-once-garantie. Verwerk dubbele en uit-volgorde berichten en crashes tussen acceptatie en lokale opslag.

Synchroniseer bij startup/reconnect balances, posities, open orders en recente fills via snapshots plus updates voordat nieuwe entries toegestaan zijn. Detecteer externe orders. Laat slechts één gateway per account orders plaatsen; voorkom split-brain tussen lokaal en VPS met afgedwongen eigenaarschap, lease/fencing en geblokkeerde toegang voor de niet-actieve omgeving.

Gebruik exchange-native bescherming waar het account en ordertype dit ondersteunen. Controleer bescherming na partial fills en amendments. Gebruik reduce-only waar geldig; begrens spot-verkopen op beschikbare inventaris. Behandel Cancel All After als scoped annulering, niet als positie-sluiting, en controleer gevolgen voor beschermingsorders.

## 12. Paper, OKX Demo en backtesting

Maak drie strikt gescheiden modi: `PAPER_LOCAL`, `OKX_DEMO` en `LIVE`. Scheid credentials, execution-adapters, opslagnamespace, balances en dashboards. Lokale simulatie mag technisch geen live-orderpad kunnen bereiken. Verander een actieve bot met open exposure niet met één toggle van omgeving; maak een afzonderlijke deployment.

Gebruik dezelfde strategie-, Pine- en risk-logica voor replay, paper en live. Alleen data- en execution-adapters verschillen. Modelleer spread, actuele maker/taker-fees, partial fills, slippage, vertraging, minimumorders en waar relevant funding. Een limit-aanraking is niet automatisch een fill.

Maak candle-backtests onderscheidbaar van tick/orderbook-simulaties. Een candle-close-signaal mag niet eerder in dezelfde candle gevuld worden. Behandel onduidelijke stop/target-volgorde conservatief; label geschatte queue-position expliciet als schatting. Voer synthetische chartprijzen niet uit als echte marktprijzen.

Lever replay, walk-forward/out-of-sample-evaluatie, parametergevoeligheid en rapportages met nettoresultaat, drawdown, turnover, fees, exposure en onzekerheid. Gebruik uitsluitend destijds beschikbare informatie. Houd bij AI rekening met kennis uit latere trainingsdata: een historische AI-backtest alleen bewijst geen voorspellend vermogen. Bevestig gedrag via forward paper trading en vergelijk met dezelfde strategie zonder Jev.

## 13. Nieuws, sentiment en aanvullende informatie

Integreer afzonderlijke provideradapters: OKX-announcements/status voor exchange-events, CryptoPanic als te onderzoeken nieuwsbron, CoinGecko voor metadata/marktcontext en Alternative.me voor de Bitcoin Fear & Greed Index. Controleer abonnement, gebruiksrechten, quota, dekking en werkelijke verversingsfrequentie vóór implementatie. Verzin geen ongedocumenteerde endpoints.

Toon bron, publicatietijd, eerste ontvangst, betrokken assets, relevantie, dataversheid en eventueel modelsentiment. Dedupliceer dezelfde gebeurtenis over meerdere bronnen. Presenteer een langzaam bijgewerkte Bitcoin-index niet als realtime sentimentscore voor iedere altcoin.

Houd trage externe feeds buiten de snelle orderroute; gebruik gecontroleerde snapshots met TTL. Nieuws is onbetrouwbare input, nooit een instructie aan het systeem. Filters tegen prompt injection, HTML/script-injectie en SSRF zijn verplicht. Ontbrekende data blijft zichtbaar ontbrekend.

## 14. Meetbare snelheid

Instrumenteer event-ontvangst, featureberekening, scriptduur, Jev-roundtrip, risk checks, duurzame registratie, verzending, acknowledgement en fill afzonderlijk met p50/p95/p99. Geef verschillen tussen lokale verwerking en netwerk-/exchangevertraging helder weer. Exchangetimestamps zonder klokcorrectie bewijzen geen exacte one-way-latency.

Gebruik als te toetsen eerste doel p95 onder 10 ms voor een eenvoudige lokale native strategie van ontvangen event tot gevalideerde intent, exclusief externe AI/netwerk en duurzame orderregistratie. Meet daarnaast de volledige werkelijke orderroute inclusief alle vereiste stappen. Dit doel is een benchmarkcriterium, geen reeds behaalde belofte.

Publiceer hardware, instrumentenaantal, events per seconde, actieve bots, indicatorlast en foutpercentages bij iedere benchmark. Profileer native, Pine- en Jev-routes afzonderlijk. Test normale én burstbelasting, geheugenontwikkeling en herstel.

Throttle alleen UI-weergave; verlies geen kritieke order- of risicoberichten. Gebruik bounded queues en expliciete overload-policies. Kies VPS-locatie met gemeten latency naar toegestane OKX- en AI-endpoints, niet op marketingclaims. Optimaliseer pas na profiling; beperk geen veiligheid om een fraai snelheidsgetal te halen.

## 15. Beveiliging en 24/7 Docker-operatie

Ondersteun Linux amd64/arm64 en lokale Docker-ontwikkeling. Lever Compose-profielen voor development, paper, productie en optionele observability. De productiekern moet blijven draaien zonder frontend. Eén VPS is geen high-availability-oplossing: documenteer storingen, herstel en resterende exchange-bescherming.

Gebruik uitsluitend noodzakelijke exchange-rechten, zonder withdrawals en met IP-binding waar mogelijk. Laat de gateway alleen expliciet toegestane API-operaties uitvoeren. Stop secrets nooit in frontend, Git, image-layers, URL-logs of AI-inputs. Versleutel opgeslagen credentials en backups met sleutels buiten de database; documenteer rotatie, herstel en hostbeveiliging. Docker Compose secrets zijn niet vanzelf versleutelde opslag.

Gebruik lokaal standaard paper-only credentials; houd live-credentials uitsluitend bij de actieve gateway. Een lokale UI mag de VPS beheren zonder een tweede live-executor te starten. Plaats de terminal bij voorkeur achter een private VPN, of anders een beveiligde TLS-reverse-proxy met sterke authenticatie en MFA/passkeys. Implementeer sessieverval, WebSocket-origincontrole, CSRF/CSP, inputvalidatie en rate limiting. Publiceer databases en interne workers niet op internet.

Containers draaien non-root met minimale rechten, read-only filesystem waar mogelijk en resource limits. Mount geen Docker-socket in appcontainers. Scan dependencies en images; documenteer de threat model voor scripts, nieuws, browser, secrets en execution-gateway.

Lever health/readiness checks, metrics, gestructureerde logs, diskbewaking, versleutelde backups en een geteste restoreprocedure. Alert op disconnects, onbekende orders, ontbrekende bescherming, drawdown, foutpercentages en storageproblemen. Updates gebeuren gecontroleerd met expliciet positie- en orderbeleid, niet als blind automatisch herstarten.

## 16. Implementatiefasen, tests en oplevering

Werk incrementeel zonder de eindscope stilzwijgend te verkleinen. Bouw eerst een veilige verticale doorsnede, daarna de geavanceerde functies.

Fase 0: verifieer API's, accountbeperkingen, licenties en runtimekeuzes; lever architecture decisions, threat model, capability-matrix en acceptatieplan. Fase 1: Docker-startbare read-only terminal met echte OKX-historie/live-data, charts, indicators en verbindingsstatus. Fase 2: centrale risk/OMS-kern, lokale paper-adapter, eenvoudige bot en replay. Fase 3: uitgebreide chart-tools, serveralerts, Pine-editor/runtime en Bot Studio. Fase 4: Jev, nieuws/sentiment, kostenmeting en vergelijkende evaluatie. Fase 5: OKX Demo, foutinjectie, hersteltests, security-review en expliciet vrij te geven live-adapter.

Test minimaal sequence-gaps, oude candles, reconnects, volle queues, Jev-timeouts/late antwoorden, dubbele fills, partial fills, ordertimeout na acceptatie, procescrash, database-uitval, disk-full, klokafwijking, rate limits, bots die hetzelfde budget claimen, gelijktijdige lokale/VPS-executors, stopbescherming en verlieslimieten na herstart. Voeg tests toe dat paper nooit live-orders kan plaatsen en ongeldige Pine-code nooit de gateway kan bereiken.

Live blijft uit tot acceptatiecriteria aantoonbaar gehaald zijn en ik bewust het account, instrumenten, budget en limieten bevestig. Een geslaagde technische test is geen bewijs van winstgevendheid.

Lever broncode, migrations, Dockerfiles/Compose, `.env.example` zonder secrets, installatie- en herstelhandleiding, API-documentatie, architectuur, threat model, Pine-compatibiliteitsmatrix, provider-/licentieoverzicht, tests, echte benchmarkresultaten en een statuslijst met werkend/niet-getest/geblokkeerd/gepland. Geen mockknoppen of TODO's presenteren als afgeronde functionaliteit.

Begin direct met fase 0 en daarna de uitvoerbare basis van fase 1. Gebruik gemotiveerde veilige aannames wanneer niet-kritieke keuzes ontbreken. Ontbrekende credentials of licenties blokkeren de betreffende integratie of livegang, niet alle overige ontwikkeling. Rapporteer eerlijk wat werkelijk gebouwd, uitgevoerd en getest is.

---

# Bronnen bij deze bouwprompt

De onderstaande verwijzingen zijn op 2 oktober 2026 geraadpleegd. Actuele providerdocumentatie en de werkelijk geconfigureerde accountrechten blijven leidend. De technische architectuur, risicopolicies en benchmarkdoelen hierboven zijn voorgestelde ontwerpeisen, geen claims dat leveranciers deze garanties leveren.

1. OKX EEA API V5 — endpoints, kanalen, authenticatie, demo en ordermanagement:
   https://my.okx.com/docs-v5/en/
2. OKX API best practices:
   https://www.okx.com/docs-v5/trick_en/
3. OKX WebSocket-poortwijziging — gepubliceerd 30 september 2026; sluiting 8443 op 31 oktober 2026; poort 443 al beschikbaar:
   https://www.okx.com/en-eu/help/okx-websocket-port-8443-discontinuation-announcement
4. OKX checksum-deprecatie — productie 23 juni 2026; sequence-controle vervangt checksumcontrole:
   https://www.okx.com/help/okx-order-book-channels-checksum-field-deprecation
5. OKX historische downloads:
   https://www.okx.com/en-eu/historical-data
6. TradingView chartbibliotheken en beperkingen voor privégebruik:
   https://www.tradingview.com/free-charting-libraries/
7. Geen ingebouwde Pine-uitvoering in Advanced Charts/Trading Platform:
   https://www.tradingview.com/charting-library-docs/latest/resources/Frequently-Asked-Questions/
8. Lightweight Charts:
   https://github.com/tradingview/lightweight-charts
   https://tradingview.github.io/lightweight-charts/docs/plugins/intro
9. Pine execution model en repainting:
   https://www.tradingview.com/pine-script-docs/language/execution-model/
   https://www.tradingview.com/pine-script-docs/concepts/repainting/
10. PineTS — huidige mogelijkheden, roadmap en AGPL/commerciële licentie:
    https://github.com/LuxAlgo/PineTS
11. DeMARK — indicatorbeschrijvingen en officiële Sequential-informatie:
    https://demark.com/indicators-list/
    https://demark.com/sequential-indicator/
12. TypeSafe introductie, beperkingen en confidence:
    https://docs.typesafe.ai/introduction
    https://docs.typesafe.ai/model-jaggedness/jev-1.13
    https://docs.typesafe.ai/confidence
13. Jev via OpenRouter — API, SDK-route en model:
    https://openrouter.ai/docs/guides/community/jev
    https://openrouter.ai/docs/guides/community/jev-tutorial
    https://openrouter.ai/docs/guides/community/typesafe-sdk
    https://openrouter.ai/typesafe/jev-1.13
14. Aanvullende data:
    https://cryptopanic.com/developers/api/
    https://cryptopanic.com/guides/how-to-integrate-the-cryptopanic-api
    https://docs.coingecko.com/demo/reference/endpoint-overview
    https://alternative.me/crypto/fear-and-greed-index/
    Let op: de CryptoPanic-documentatie was publiek slechts beperkt uitleesbaar; exact plan, endpoints, schema's, rechten en quota moeten tijdens implementatie worden bevestigd.
15. Voorgestelde ontwikkelcomponenten:
    https://react.dev/
    https://vite.dev/guide/
    https://tokio.rs/tokio/tutorial
    https://reactflow.dev/
    https://github.com/microsoft/monaco-editor
16. Grenzen van stoporders en uitvoeringszekerheid:
    https://www.okx.com/en-eu/help/why-didnt-the-tp-sl-execute-at-the-set-price
    https://www.okx.com/en-eu/help/how-do-i-trade-with-different-order-types
