# Acceptatie tegen de volledige bouwprompt

Willem heeft gevraagd alle verdere fasen te bouwen. Dit document beschrijft het werkelijk behaalde bewijs; het is **geen verklaring dat de volledige opdracht of fase5 is afgerond**. Account-, instrument-, budget- en limietkeuzes blijven voor Willem. Jev heeft nooit uitvoeringsautoriteit.

## Update 6 oktober 2026 · draagbare netwerkexport en import

Training Lab bevat Netwerkarchief: JSON-export en persistente import van vastgezette kandidaten, het nieuwste tussentijdse netwerk en eerdere generatiecheckpoints. Alle 340 gewichten/biases, normalisatie, parameters en model-/dataversies blijven behouden. Export kan na de eerste berekende generatie, vóór volledige afronding. Import is een onderzoekskopie zonder handelsvrijgave of overgenomen testbewijzen; volledige optimizerhervatting blijft in de oorspronkelijke campagne.

254 Rust-tests, TypeScript/productiebuild en 20 geïsoleerde portabilitycontroles slagen. De browserproef controleert werkelijke bestandsdownloads, een opgeslagen netwerk terugladen, een eerdere generatie en de matrices. Gewijzigde bestanden zijn via de API afgewezen. De aanvullende uploadproef via de browser is niet uitgevoerd: de uploadtoestemming werd geweigerd; er is geen omweg geprobeerd. De testgegevens zijn kunstmatig, uitsluitend in een wegwerpdatabase. Alleen de terminal is bijgewerkt; beide gateways, trainer, rekeningpolicy en persoonlijke vrijgave zijn behouden. [Bediening](TRAINING_LAB.md#netwerk-exporteren-en-importeren), [bestands-/databaseproef](../evidence/training-transfer-integration.json), [behoudcontrole](../evidence/training-transfer-local.json).

## Training Lab en native neurobotroute · 6 oktober 2026

Onderzoek, Laya/CMA-ES, dataset-/artifactversies, eenmalige kwalificatie, forward en netwerkvisuals zijn lokaal gebouwd. De native route bevat persoonlijke exacte-versievrijgave, rekeningbrede reserveringen met handmatige orders, eigen botinventaris/fees/risico, native bescherming, vaste houdduurexits en gatewayhercontrole. 248 Rust-tests, zes trainertests, twintig onderzoekscontroles en achttien native wegwerpdatabasecontroles slagen. De laatste proef gebruikt expliciet kunstmatige bewijs- en accountgegevens en verzendt geen private exchangeorders.

**Geen volledige acceptatie:** geen gekwalificeerde bot, geen werkelijk 30 dagen/100 forwardtrades, geen echte neurobot-Demo-acceptatie en geen volledige native duur-/resource-/latencyproef. De OCO-annulering → begrensde IOC bij maximale houdduur moet ook onder partial fills, timeout en herstart op Demo worden bewezen. De bestaande OKX-IP-weigering blijft een accountblokkade. Er is geen gebruikersbudget of botvrijgave ingesteld. [Actuele labhandleiding](TRAINING_LAB.md).

Na expliciete toestemming van Willem zijn op 6 oktober om 14:39 beide bestaande Demo- en Live-gateways vervangen door de gebouwde en geteste versie. De lopende gateways en terminal hebben dezelfde uitvoeringscodehash. Budgetten, limieten, persoonlijke Live-vrijgave en order-/fillaantallen zijn behouden; terminal, trainer en database zijn niet herstart. Demo mist nog sleutels; Live blijft gepauzeerd wegens OKX 50110 / IP-toelating. Geen neurobot vrijgegeven. [Installatie- en behoudbewijs](../evidence/training-gateway-update.json).

## EUR/USDC-uitbreiding · 6 oktober 2026

Willem heeft de uitbreiding naar alle EUR- en USDC-spotmarkten van zijn eigen OKX-account expliciet toegestaan. De actuele bron is gericht samengevoegd en de terminal plus beide afzonderlijke ordergateways zijn bijgewerkt. De eerdere grens van twaalf persoonlijk gekozen markten is verwijderd. Budget, limieten, acht gekozen EUR-markten en de bestaande persoonlijke Live-vrijgave zijn behouden; geen nieuwe markt of order is namens Willem bevestigd.

Het account-endpoint blijft de bron voor beschikbare markten. **Alle beschikbare spotmarkten kiezen** selecteert de volledige EUR/USDC-accountlijst in het formulierconcept. De gebruiker bevestigt vervolgens zelf de limieten en geeft Live opnieuw vrij voor die nieuwe versie. Prijsvelden, previews en kooppercentages gebruiken de werkelijke betaalvaluta. EUR-budget en risico gebruiken actuele marktconversies. Een ontbrekende/verouderde conversie of exacte marktfee blokkeert de aankoop. Kopen met USDC en verkopen van USDC delen dezelfde saldoreservering; crypto-inventaris en blootstelling worden over EUR/USDC-paren van dezelfde munt samengenomen.

**Actuele accountcontrole is nog geblokkeerd:** OKX antwoordt HTTP 401 / code 50110 op `/api/v5/account/bills`, en de private login wordt eveneens geweigerd. De terminal toont dat het huidige IP-adres niet door de API-key wordt toegelaten. De IP-lijst moet door Willem bij OKX worden gecontroleerd. Er is geen verse EUR/USDC-accountlijst geclaimd: de oude lijst van 270 EUR-markten is verouderd. De gateway blijft gepauzeerd en de UI toont geen onjuiste marktweigering of actieve Live-verbinding op basis van dat oude snapshot.

Bewijs: productiebuild en TypeScript; 211 Rust-tests (32 onboarding, 44 gateway, 135 terminal); 32 bestaande native HTTP-controles en 25 aanvullende EUR/USDC-controles in een wegwerpdatabase. De tweede productiebrowserproef bevestigt USDC-prijsunits, grafiekklik met hoeveelheidbehoud, de IP-melding, en geen consolefouten. De fictieve browserrekening bevestigt wissen/alle kiezen, USDC-zoeken en uitgeschakelde opslag zonder eigen bevestiging. De sliders blijven in de browser uitgeschakeld zonder geconfigureerde keys; hun 50%-USDC-berekening is via de geïsoleerde HTTP-route getest. Geen echte testorder, transfer of gebruikersinstelling gewijzigd. Dit is geen volledige native exchange- of fase-5-acceptatie.

Bewijsbestanden: `evidence/eur-usdc-*`. Herhaalbare controles: `scripts/order_acceptance.py provision`, `scripts/eur_usdc_checks.py`, daarna `scripts/order_acceptance.py cleanup`. De opname verwijst uitsluitend naar de wegwerpomgeving op poort 4781.

## Stand per fase

| Fase | Stand |
| --- | --- |
| 0 | Architectuur, threat model, providers/licenties, scope en capabilities; persoonlijke accountrechten nog onbekend |
| 1 | Echte publieke terminal, charts/indicatoren/status; uitgebreid met sequenceboek en multi-chart |
| 2 | Centrale risk/OMS, lokale paperadapter, serverbot en replay; herhaalde37 integratiechecks |
| 3 | Pine closed-bar, visuele Bot Studio, tekeningen en serveralerts gebouwd/getest; geavanceerde Pine/docking volgen nog |
| 4 | Jev Decisions-adapter/kosten/fixtures en echte nieuws/FNG-adapters, OOS en forwardvergelijkingsrapport gebouwd; echte Jev-call/vergelijking nog niet uitgevoerd |
| 5 | Securityhardening/auth/backuprestore en read-only Demo-onboarding gebouwd; operationele exchangegateway/reconciliatie/bescherming en accountacceptatie ontbreken |

## Bewijs en foutmatrix

94 Rust-tests + 11 Pine-tests; 87 geïsoleerde HTTP/integratiecontroles (37+26+14+10). Geen gebruikersrekening of echte orders. Bronbewijs staat in STATUS.md/evidence. Scheid fixture van realistische adapterproef:

| Eigenschap | Werkelijk getest | Nog open |
| --- | --- | --- |
| Book-data | Snapshot/delta seq/prev, heartbeat/reset/gap, echte publieke stream | Bulk tick/L2-dekking |
| Oude/gappende candles | Normalisatie, warm-up, gesloten bars, next-bar replay | Business-WS/intrabar/securityprofielen |
| Risk/gelijktijdig budget | Decimal/rekeningrowlock, één winnaar, bot-/accountlimieten | Echte fees/cashflows/conversies/subaccounts |
| OMS/partial/dedup | Lokale ACK/fill apart, reserves, crash/DB-uitval, verliezen na herstart | Exchangeacceptatie gevolgd door timeout en echte externe orders |
| Twee executors | Tweede lokale paperworker geweigerd | Live-credential/egress-fencing tussen hosts |
| Pine | Foutcode geeft geen intents, toekomstige data/ongeldige TA/inputs geweigerd, geïsoleerde processen | Breed adversarial corpus en verdere taalprofielen |
| Jev | Responseschema, state/TTL/confidence/kostenregels in fixtures; geen key geeft weigering | Echte providerresponse/timeout/late-response-kostenacceptatie en forwardvergelijking |
| Alerts | Serverprijs/lijn/bot/inboxdedup; zonder browser | Externe notificatiedienst niet gevraagd/ingesteld |
| Storage/auth | Drempel/fail-closed unit, AES-GCM-scope/tamper, DB+Parquet-restore, auth/expiry/logout/CSRF | Fysieke full-disk/hostdisaster en remote MFA/TLS |
| Performance | Native fixture, echte WS/HTTP, begrensde120-requestburst, Pinepreviewduur | Lange soak, volledige event→intent/submit→ack/fill-percentielen |
| Supply chain | Rust/npm en drie runtimeimages0 meldingen na hardening | Onafhankelijke review en onbekende kwetsbaarheden |

## Traceability

| Specsectie | Huidige invulling | Open grens |
| --- | --- | --- |
| 1–2 | Nieuw lokaal spotproject, Rust/React/PG/Parquet, execution-procesgrens | Geen cloud; echte gateway nog open |
| 3 | Werkruimtes, multichart, workspaces, chartlijnen/preview, commandpalette | Vrije docking, orderlijn-amendments, snelmodus |
| 4–5 | Native publieke EEA, sequence/gaps, historie/coverage, metadata/klok | Persoonlijke rechten/private reconcile, history-trades/business-WS |
| 6 | Tien instelbare indicatoren, negen tekentools en serveralerts | DeMARK, volumeprofile/orderflow/heatmaps |
| 7 | Echte parser/AST/Monaco/inputs/versies, geïsoleerd closed-bar-profiel | Verdere Pine-semantiek zoals expliciet in matrix |
| 8–9 | Canonieke graphcompiler, templates, serverdeployments, Jev-proposals/costguard | Echte Jev-acceptatie, later gecontroleerde livepromotie |
| 10–11 | Centrale paper-risk/OMS/stop/reserves/HWM/lease | Echte rekeningadministratie, exchange-native bescherming/fencing |
| 12 | Replay/Pine/walk-forward/OOS/gevoeligheid en forwardrapport | Demo/LIVE-acceptatie, tickboekreplay |
| 13 | OKX-news/status en dagelijkse FNG, bron/TTLstatus | CryptoPanic/CoinGecko plan/rechten |
| 14–15 | Echte beperkte metingen, localhost, encryptie/auth/scans/backuprestore/logrotatie | Langdurigeoperatie/amd64/remote beveiliging |
| 16 | Bron/locks/migraties/Docker/handleidingen/tests/status | Volledige exchangefoutmatrix en bewuste livevrijgave |

Concrete vervolgstappen en implementatiehiaten: [REMAINING.md](REMAINING.md). Die scope is niet stilzwijgend vervallen.

## Aanvullende UI-acceptatie · 4 oktober 2026

[De tekenbediening](DRAWING_TOOLS.md) is herzien en in de browser getoetst voor alle negen vormen, selectie/slepen, undo/redo, opslag, live verversing, thema's en multichart. Tien gerichte frontendtests en de productiebuild slagen. Voor het exacte bewijs en de testgrenzen zie de handleiding; dit verandert de open exchange-/fase5-acceptatie niet.

## Aanvullende tijdframeacceptatie · 4 oktober 2026

Veertien grafiekintervallen, inclusief UTC-week, kalendermaand en kwartaal, zijn lokaal actief. De volledige Rust-suite slaagt met 100 tests (91 + 9); de tekengeometriesuite telt nu 11 geslaagde tests. TypeScript en de productiebuild slagen. De publieke dataproef heeft alle veertien intervallen zonder gaten en met actuele data gecontroleerd, plus historische week-/maandpaginatie. Browsercontrole bevestigt week-/maand-/kwartaalweergave en het bewaren van de maandkeuze na herladen. Zie [TIMEFRAMES.md](TIMEFRAMES.md) voor calendertests, databeperkingen en bewijs.

## Geïntegreerde Bot Studio · 5 oktober 2026

- 109 Rust-tests slagen, inclusief 9 gerichte tests voor if/else, ongeldige handles, niet-gekozen takken, NOT, cooldown en Pine-plotvoorwaarden.
- 26 disposable API-proeven slagen: complete botconfiguratie bewaren/herladen, versieconflicten, atomair gepauzeerd toepassen, Live opslaan maar installatie blokkeren (ook legacy endpoint), volledige graphreplay met behoud van limieten, geen fills vóór de signaalcandle, blijvende rapporten, exacte Pine-versies/inputs, indicatorplots als voorwaarden, geen stil afgekapt signaal en afwijzing van ongeldige budgetten. Gebruikersrekening is ongewijzigd.
- 11 bestaande Pine-fixtures, 11 charttekentests en frontendproductiebuild slagen.
- Interactieve browserproef op disposable poort 4781: if/else-drempel wijzigen, bot kopiëren en opslaan, Live-ontwerp historisch simuleren, Pine numerieke input aanpassen en script op koerschart laden, bots zichtbaar in terminal. Geen consolefouten tijdens deze proef.
- Geen account, budget, betaalde Jev-call of botstart in de gebruikersomgeving geconfigureerd. LIVE/Demo-fase 5, volledige Pine-strategiesemantiek en forward-/historische statepariteit blijven afzonderlijk open; zie [BOT_STUDIO.md](BOT_STUDIO.md).

De definitieve versie is healthy op poort 4780. Aanvullend gecontroleerd: vier Pine-charts met gedeelde previewwachtrij, correcte opgeslagen-status na herladen, Pine-editor → geselecteerde botnode en een verse servercandle die in de disposable rekening een BUY via de vaste paperkern opleverde. Testbot daarna gepauzeerd; gebruikersrekening blijft niet ingesteld.

## Systeem-instellingen · 5 oktober 2026

Systeem opent met Aansluitingen & AI: gescheiden OKX Demo-/Live-keys (key, secret, passphrase), Jev direct via TypeSafe en Jev via OpenRouter. Sleutels kunnen lokaal versleuteld worden opgeslagen, vervangen en verwijderd. De opgeslagen providerkeuze bepaalt de daadwerkelijke Jev-route. Budgetbediening en kostenoverzicht zijn hier beschikbaar; een provider- of AI-keywijziging via Systeem schakelt AI uit tot herbevestiging. Bestaande systeemmetingen staan onder Status & bescherming.

114 Rust-tests, de productiebuild en 40 geïsoleerde integratiecontroles slagen. Browserbediening, herladen, lege keyvelden na opslaan en lichte/donkere weergave gecontroleerd. Tests gebruikten alleen fictieve credentials in een tijdelijke database. Geen echte provideracceptatie, betaalde calls of orders uitgevoerd. TypeSafe-kosten zijn expliciet berekend uit ontvangen tokengebruik tegen het vastgelegde openbare tarief. Zie [gebruik, opslag en beperkingen](SYSTEM_SETTINGS.md) en `evidence/system-settings-integration.json`.

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

De door Willem aangeleverde Pine v5-indicator is omgezet naar het bestaande numerieke profiel, zonder terminalcode te wijzigen. Bron: `pine-scripts/913sig-okx.pine`; opgeslagen als **913sig OKX · v1**, type indicator, eigen paneel. Dalende/stijgende plots bevatten 0 of het oorspronkelijke labelnummer 7/8/9/11/12/13. Setup- en bonusresets blijven gelijk aan het origineel. Pine-labels boven/onder candles en hun afzonderlijke kleuren zijn niet behouden; zie [weergave en gebruik](913sig-okx.md).

Negen gerichte berekeningstests slagen tegen de oorspronkelijke na-/labelvoorwaarden, inclusief 5.000 candles en bonusonderbreking terwijl setup doorloopt. HTTP-preview en de browsereditor slagen op 299 actuele gesloten BTC-EUR-candles van 15 minuten. De exacte opgeslagen bron is teruggelezen en geverifieerd. Bewijs: `evidence/913sig-verification.json`, `evidence/913sig-http-verification.json`, `evidence/913sig-editor.jpg`. Geen entry/exit-plots, handelsvoorstellen, botstart of rekeningwijziging door deze omzetting. Geen brede Pine- of TradingView-pariteitsclaim.


## Native orderticket en Demo-gateway · 5 oktober 2026

- Geslaagd: 174 Rust-testuitvoeringen, frontendproductiebuild, 32 geïsoleerde HTTP/DB-controles (alle tien native ordercontracten, concurrent inventaris, idempotentie, herstart, annuleren, verlopen preview, CSRF/Live-blokkade, fee-aware filldedup en beschermingsclaims), tien browserformulieren en TWAP-preview. Bewijs: `evidence/native-orders-rust-tests.txt`, `native-orders-integration.json`, `native-orders-browser.json`.
- Lokaal geactiveerd: terminal en afzonderlijke Demo-gateway; KEY_REQUIRED, nul native orders, gebruikersrekening-/botlimieten behouden. `evidence/native-orders-local.json`.
- Niet geaccepteerd: daadwerkelijke EEA/Demo-endpoints, ordertype-/instrumentrechten, continue private kanaaluitval, native child-/partialfillbescherming, cancelrace, account-/cashflowherstel en exclusieve gateway in echte netwerkfouten. Geen keys aanwezig. Live blijft uit.
- Implementatiegrenzen staan afzonderlijk in [ORDERS.md](ORDERS.md), waaronder amend, derivaten, quote-sizing, gezamenlijke bot-/native-ledger, afbouw/replacement en grote historiepaginering. Lokale tests bewijzen geen native uitvoering of volledige OKX-pariteit.


## Live-saldi, percentagebediening en persoonlijke vrijgave · 5 oktober 2026

- 191 Rust-testuitvoeringen slagen (25 onboarding, 37 ordergateway, 129 terminal), plus TypeScript en productiebuild. De 58 geïsoleerde HTTP/DB-controles bestaan uit 32 bestaande native-ordercontroles en 26 saldi-/percentage-/vrijgavecontroles. Zij gebruiken fictieve accountgegevens zonder private exchange-calls.
- De echte Live-aansluiting is uitsluitend lezend gecontroleerd: actuele EUR/BTC/USDC-handelsaldi, Funding succesvol leeg, lees-/handelsrechten, private én business-kanaal verbonden en gereconcilieerde administratie. Normale channel-conn-count-berichten worden onderscheiden van subscribe-ACK; fouten weigeren uitvoering.
- Browser en server bevestigen 50% van het vrije BTC-saldo, naar beneden afgerond op de lotgrootte. Handmatige invoer wist het percentage; richting/account/markt-wissel wist de hoeveelheid; Demo zonder key schakelt de slider uit. Handelsrekening en Funding blijven gescheiden.
- De terminal, afzonderlijke Demo-gateway en afzonderlijke Live-gateway draaien lokaal. De bestaande virtuele rekening en botlimieten zijn behouden. De gebruiker heeft nog geen persoonlijke Live-limieten of vrijgave bevestigd. Geen echte orders geplaatst, geen transfers en geen bedragen namens Willem gekozen.
- De algemene technische Live-blokkade is op uitdrukkelijk verzoek vervangen door persoonlijke vrijgave voor exact account en limietversie. De echte native orderacceptatie blijft open; technische acceptatievlaggen blijven false. Alle eerdere open punten voor partial fills, bescherming, herstel en geavanceerde children blijven van toepassing.
- Uitvoering blijft EUR spot. USDC-saldo ophalen werkt; toepassing van USDC-orderuitvoering is door automatische goedkeuringscontrole geweigerd wegens ontbrekende expliciete toestemming voor uitbreiding van handelsmarkten. Die afzonderlijke keuze is aan Willem voorgelegd en staat nog open. De voorbereide uitbreiding is niet geïnstalleerd.

Bewijs: `evidence/live-balances-rust-tests.txt`, `live-balances-integration.json`, `live-balances-build.txt`, `live-balances-local.json`, `live-balances-source.json` en browserbewijs. Lokale controles betekenen geen volledige OKX-pariteit of aanvaarde daadwerkelijke orderuitvoering.


## Live-vrijgavebediening · 5 oktober 2026

De echte Live-verbinding, handelsrechten en reconciliatie waren actueel, maar policy en live_release ontbraken. Daardoor bleef de vrijgaveknop uitgeschakeld zonder concrete uitleg. Het ticket toont nu elke ontbrekende voorwaarde en biedt **Account en handelslimieten instellen**. Deze knop klapt het bestaande formulier open, scrollt ernaartoe en richt de focus op het budget. Accountidentiteit, persoonlijke bevestiging en beschikbare markten worden ook bij de weergave getoetst; de backend blijft de uitvoeringsbeslissing nemen.

TypeScript en productiebuild slagen. De productie-browserproef bevestigt de reden, directe formulierroute, leeg gebleven persoonlijk budget en uitgeschakelde Live-confirmatie zonder limieten. Alleen de terminal is opnieuw gestart; de bestaande private gateways bleven draaien. Geen accountlimieten, keys, vrijgave of orders namens Willem gewijzigd. Bewijs: `evidence/live-release-help-*`. Dit is een bedieningscorrectie; daadwerkelijke native orderacceptatie en USDC-uitbreiding blijven afzonderlijk open.


## Automatische voorbeeldlimieten · 5 oktober 2026

Op verzoek rekent het limietenformulier na budgetinvoer met een aanpasbaar startprofiel. Zeven gerichte tests slagen: complete EUR-500-voorbeelden, exacte decimalen, wijzigingen met behoud van eigen limieten, ongeldig/gewist budget zonder oude automatische bedragen, expliciet herstellen, behoud van opgeslagen persoonlijke limieten en bewust leeggemaakte eigen velden. TypeScript en productiebuild slagen. Browserproef bevestigt alle tien waarden bij EUR 500, een eigen EUR-3-risico bij budget EUR 1000, herstellen naar het profiel, gescheiden Live-/Demo-concepten en vereiste persoonlijke bevestiging. De agent heeft geen beleid opgeslagen, vrijgave uitgevoerd of order geplaatst. Tijdens de controle zijn via de bestaande bevestigingsroute twee limietversies opgeslagen en is Live vrijgegeven. Daarom is er geen claim dat de voor/na-beleidshashes gelijk zijn. De audit bevat LIMITS_CONFIRMED op 1791232681969 en 1791232686652, en LIVE_ENABLED op 1791232694288. Het aantal native orders is nog nul; de gateways zijn verbonden/gereconcilieerd. Persoonlijke vrijgave is geen daadwerkelijke orderacceptatie.

Bewijs: `evidence/limit-suggestions-*`. Dit is invoerhulp. De eerdere open native orderacceptatie, Live-/USDC-marktrechten en fase-5-implementatie blijven afzonderlijk staan.


## Grafiekklik naar limietprijs · 5 oktober 2026

Vier gerichte prijs-/ordertypetests, zeven bestaande limieten- en elf tekenmodeltests, TypeScript en productiebuild slagen. De productie-browserproef bevestigt klikprijzen op twee hoogten, behoud van hoeveelheid/stop bij kopen, verkopen, vaste hoeveelheid na percentageslider, geen prijswijziging door volume/RSI/slepen/tekenen, undo van de tijdelijke proeftekening, marktklik zonder terugwerkende toepassing, triggerprijsbehoud, prijsreset bij marktkeuze en de tweede gekoppelde grafiek. Geen consolewaarschuwingen of -fouten.

Alleen de lokale interface is vernieuwd; de backendbinary is identiek en de afzonderlijke Demo-/Live-gateways bleven draaien. Beleid, persoonlijke Live-vrijgave en native orderaantal zijn voor/na lezend gecontroleerd in `evidence/chart-price-local.json`; de browserproef bevat geen preview/submit of accountwijziging. De grafiekactie past uitsluitend het actieve formulierconcept aan. De eerder open daadwerkelijke native orderacceptatie, USDC-uitbreiding en fase-5-grenzen blijven afzonderlijk staan. Bewijs: `evidence/chart-price-*`.


## Markttoegang en diagnose · 5 oktober 2026

De publieke catalogus is nu volledig doorzoekbaar: bij controle 1143 live spotparen, waaronder 271 EUR- en 298 USDC-paren. Publieke zichtbaarheid is geen accounttoestemming. Grafieken buiten de oude twaalf paren worden op verzoek gekoppeld aan de bestaande publieke boek-/ticker-/tradevalidatie; actieve feeds blijven begrensd. Browserproef bevestigt NEAR-EUR met actuele candles en boek.

Het orderticket onderscheidt een beschikbare EUR-markt die niet persoonlijk is bevestigd van de terminalbeperking op een andere betaalvaluta. Het limietenformulier toont de EUR-markten uit de lopende accountgateway met een zoeker. De huidige grens van twaalf tegelijk blijft staan. Bij controle waren acht EUR-markten persoonlijk bevestigd en gaf de lopende, nog EUR-filterende gateway 270 EUR-paren terug. De publiek zichtbare andere valuta's worden niet ten onrechte als ontbrekende OKX-rechten voorgesteld.

De brede Live-/risico-uitbreiding werd door automatische goedkeuringscontrole geweigerd wegens ontbrekende specifieke markt-/valutatoestemming. Een diagnostische gatewayherstart werd afzonderlijk geweigerd wegens mogelijke verstoring van de Live-service. Daarom is uitsluitend de terminal vernieuwd; beide private gateways bleven draaien. De wijziging om de volledige accountlijst te lezen staat in de gatewaybron maar is daar nog niet actief. De ordercompiler, wallet- en risicokern blijven EUR-only. De gerichte EUR/USDC-uitbreiding is apart voorbereid en geïsoleerd typechecked/getest; zie [het concrete voorstel](MARKET_ACCESS_PROPOSAL.md).

De productiebuild, TypeScript, 191 bestaande Rust-tests en vier chartprijstests slagen. Browsercontrole bevestigt publieke zoekbaarheid, live feed voor NEAR-EUR, de melding over ontbrekende persoonlijke marktbevestiging, directe formulierroute, NEAR ongekozen, acht opgeslagen markten en uitgeschakelde bevestiging. Het USDC-ticket meldt de terminalbeperking. Geen consolefouten of -waarschuwingen. Beleid en persoonlijke Live-vrijgave zijn behouden. Bij controle stond één volledig uitgevoerde native BTC-EUR-verkoop in de administratie; de agent heeft geen order/preview aangemaakt of bevestigd. Dit ene gebruikersresultaat is geen volledige native acceptatie. Bewijs: `evidence/market-access-*`.
