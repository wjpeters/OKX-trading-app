# Stappenplan fase 5 en volledige afronding

Datum: 4 oktober 2026. Status: **uitvoeringsplan, nog niet uitgevoerd**.

Doel: de bestaande lokale OKX Tradingterminal afbouwen volgens de volledige [BUILD_SPEC.md](../BUILD_SPEC.md). Dit plan voegt geen clouddeployment toe. Broncode, data, sleutels, backups en testbewijs blijven buiten iCloud in het bestaande lokale project.

De opdracht van vandaag is een stappenplan. Dit document activeert geen account, bot, betaalde Jev-call, Demo-order of livehandel. Het beschrijft de bouwvolgorde en de voorwaarden voor die latere acties.

## 1. Uitgangspunt en definitie van klaar

De huidige versie 0.4.0 heeft een werkende paper-/onderzoeksbasis. De [bouwstatus](../STATUS.md), [acceptatie](ACCEPTANCE.md), [open punten](REMAINING.md), [architectuur](ARCHITECTURE.md) en relevante broncode zijn voor dit plan gelezen. Het bestaande testbewijs is van **2 oktober 2026**: 94 Rust-tests, 11 Pine-fixtures en 87 integratiecontroles. Dit is geen nieuwe runtime- of securitycontrole op 4 oktober.

Belangrijkste aangetroffen beperkingen:

| Onderdeel | Bestaande basis | Wat werkelijk ontbreekt |
| --- | --- | --- |
| Risicokern | Duurzame paper-OMS, reserveringen, verlieslimieten en herstel | Omgevingsonafhankelijke rekeningadministratie met echte fees, cashflows en externe orders |
| OKX privaat | Signer, read-only onboarding, korte kanaalprobe en fixtures | Continue synchronisatie, operationele ordergateway, bescherming en afdwingbaar exclusief eigenaarschap |
| Pine | Gevalideerd `closed-bar-1`-profiel | Verdere realtime-, multi-timeframe-, strategy- en tekenfunctionaliteit met bewijs |
| Jev | Decisions-adapter, vaste filters en kostenbewaking | Echte provideracceptatie en uitgevoerde forwardvergelijking |
| Terminal/data | Multi-chart, tekeningen, candlehistorie en replay | Vrij docking, volledige chart-trading, business-WS-candles, trades/L2 en aanvullende analyse |
| Operatie | Lokale beveiliging, scans en beperkte backup-/restoretest | Automatische grotere backups, volledig herstel, langdurige belasting en werkelijk uitgevoerde amd64-validatie |

We hanteren vier afzonderlijke mijlpalen:

- **M1: Demo werkt van begin tot eind.** Echte Demo-accountstate, orders en bescherming zijn aantoonbaar verwerkt via de vaste risicokern. Stappen 1–7 zijn geslaagd.
- **M2: volledige productscope afgewerkt.** Open functies uit eerdere fases zijn gebouwd en geaccepteerd binnen expliciete Pine-profielen en aantoonbare data-/licentierechten. Stappen 8–13 zijn geslaagd. Een geblokkeerde gewenste functie blijft open en maakt deze mijlpaal niet automatisch groen.
- **M3: technisch klaar voor gecontroleerde livevrijgave.** De live-adapter, omgevingsscheiding, security, herstel en volledige releaseacceptatie zijn aantoonbaar gereed. Stap 14 is geslaagd; LIVE staat nog uit.
- **M4: persoonlijk ingesteld en bewust vrijgegeven.** Willem bevestigt account, instrumenten, budget en limieten. Stap 15 scheidt deze persoonlijke keuze van de technische oplevering.

M1 alleen betekent dus niet dat de volledige app af is. M3 is ook geen bewijs dat de live-exchange al met echte orders is getest.

## 2. Vaste afspraken voor alle stappen

1. **Jev, Pine, bots en de browser leveren uitsluitend voorstellen.** Alleen de vaste risicokern kan een uitvoerbare intent toestaan. De enige execution-gateway verzendt die intent na een laatste geldigheidscontrole.
2. Dezelfde strategie-, Pine- en risicoregels gelden in replay, PAPER_LOCAL, OKX_DEMO en LIVE. Alleen marktdata en uitvoering verschillen per adapter. Maak geen tweede, afwijkende Demo-risicokern.
3. PAPER_LOCAL heeft geen exchange-ordercode of livecredentials. Demo en LIVE hebben afzonderlijke deployments, credentials, opslag, accountstate en dashboards.
4. Spot zonder leverage blijft het uitgangspunt. Accountrechten bepalen welke instrumenten en valuta werkelijk bruikbaar zijn. EUR/USDC in een publieke watchlist is geen bewijs dat beide met deze rekening verhandelbaar zijn.
5. Onzekere rekeningstate, onbekende orders, verlopen data, ontbrekende opslag of verloren eigenaarschap blokkeren nieuwe blootstelling. Beschermingsacties blijven waar mogelijk beschikbaar volgens een vooraf vastgelegd beleid.
6. Geen hergebruik van eerdere tradingapps, geen stilzwijgende modelwissel, geen automatische verhoging van limieten en geen herstart die risicostops wist.
7. Iedere oplevering krijgt bronversie, migratieversie, testdatum, omgeving en bewijs. Een fixture, echte Demo-test en livecontrole blijven herkenbaar verschillende bewijsvormen.

## 3. Bouwvolgorde in één overzicht

| Stap | Werkpakket | Afhankelijk van | Klaar wanneer |
| --- | --- | --- | --- |
| 1 | Accountcontract, risicoafspraken en testbasis | Huidige release | Eisen, datacontracten, capabilitystatus en reproduceerbare uitgangssituatie vastliggen |
| 2 | Gedeelde risicokern en rekeningadministratie | 1 | Echte geldstromen correct worden geboekt en paperregressies blijven slagen |
| 3 | Continue accountsynchronisatie | 1–2 | Startup/reconnect een verklaarbare actuele rekening oplevert vóór entries |
| 4 | Duurzame ordergateway | 2–3 | Plaatsen, wijzigen, annuleren en onbekende uitkomsten correct werken |
| 5 | Exchange-native bescherming | 3–4 | Iedere gevulde hoeveelheid volgens bevestigd accountbeleid wordt beschermd |
| 6 | Eén bevoegde executor en omgevingsscheiding | Ontwerp in 1, bouw met 4–5 | Een tweede of oude executor aantoonbaar geen orders kan versturen |
| 7 | Echte Demo-acceptatie en foutmatrix | 1–6, Demo-invoer | Orderlevenscyclus, storingen en herstel met bewijs zijn geaccepteerd |
| 8 | Terminalbediening en botbeheer afmaken | Contracten uit 1–4 | Alle zichtbare handels- en beheerfuncties werkelijk werken |
| 9 | Datastromen, grote historie en realistisch replay | 1–2 | Datadekking, kosten en tijdsvolgorde controleerbaar zijn |
| 10 | Pine-profielen uitbreiden | 2, 8–9 | Ieder aangeboden profiel een expliciet geslaagd referentiecorpus heeft |
| 11 | Jev, nieuws en forwardvergelijking afronden | 2, 8–9; providerinvoer voor echte tests | Werkelijke antwoorden, kosten en vergelijkingsresultaten controleerbaar zijn |
| 12 | Volume profile, orderflow, heatmaps en DeMARK | 8–9; passende data/rechten | Alleen geverifieerde berekeningen beschikbaar zijn |
| 13 | Lokale operatie, backups en herstel | Doorlopend; integratie na 3–6 en 8–12 | Een lege vervangende lokale omgeving volledig kan worden hersteld |
| 14 | Complete releaseacceptatie en live-adapter | 7–13 | Alle verplichte controles slagen en LIVE veilig uit blijft |
| 15 | Persoonlijke inrichting en bewuste vrijgave | 14, bevestigingen Willem | De gekozen omgeving gecontroleerd wordt geactiveerd en gevalideerd |

Kritieke route: **1 → 2 → 3 → 4/5/6 → 7 → 14 → 15**. Stappen 8–13 kunnen na hun genoemde afhankelijkheden onafhankelijk worden ontwikkeld. Dat is een werkverdeling, geen opdracht om nu achtergrondtaken of meerdere agents te starten.

Ontbrekende sleutels blokkeren echte account-/provideracceptatie. Ze blokkeren niet het bouwen van de gedeelde kern, gateway, fouttests, interface of herstelvoorzieningen.

## 4. Fase 5: veilige exchange-uitvoering

### Stap 1. Leg accountcontract, risicoafspraken en testbasis vast

**Bouwen en vastleggen**

- Maak een eis-tot-testlijst voor alle zestien hoofdstukken van BUILD_SPEC.md, met status `gebouwd`, `getest`, `niet getest`, `geblokkeerd` of `gepland`.
- Leg de huidige bron vast als lokale reproduceerbare Git-baseline na secretcontrole. De checkout bevat nu nog niet-getrackte projectbestanden. Geen push of externe repository is hiervoor nodig.
- Definieer contracten voor marktstate, accountstate, voorstellen, risicobesluiten, orderstatussen, fills, fees, beschermingsstatus en audit. Houd de module-interface klein en test dezelfde interface die productie gebruikt.
- Bouw een deterministische testexchange met gesimuleerde klok, vertraagde/verdwenen antwoorden, dubbele events en bestuurbare partial fills. Houd deze uitsluitend in de testomgeving.
- Breid onboarding uit tot een zichtbaar capabilityrapport per account, modus en instrument: regio, cash/spotrechten, handelsstatus, lot/tick/minimum, afrekenvaluta, fees, private kanalen, ordertypes, bescherming en requestlimieten.
- Gebruik echte nieuwe Demo-credentials pas via verborgen lokale invoer. Zolang die ontbreken, krijgen accountafhankelijke capabilities `onbevestigd`. Accountdetectie blijft eerst read-only.
- Leg exacte risicogrondslagen vast: rekeningvaluta, daggrens/tijdzone, cashflows, gerealiseerde en ongerealiseerde P&L, fees, high-water-mark, gezamenlijke muntblootstelling en onbekende waarderingen. Testbedragen horen uitsluitend bij synthetische accounts; persoonlijke limieten blijven leeg tot Willem bevestigt.

**Acceptatie:** iedere orderbepalende waarde heeft een eigenaar, bron, tijdstempel en onbekend-/verlopen-beleid. Een onbevestigde capability kan geen handelsrecht opleveren. Een reproduceerbare lokale baseline en testexchange zijn beschikbaar.

**Oplevering:** bijgewerkte architectuurbesluiten, CAPABILITIES.md, THREAT_MODEL.md en ACCEPTANCE.md; contracttests en een nieuwe lokale migratieserie. Bestaande migraties met mogelijke gebruikersdata niet overschrijven.

### Stap 2. Maak de bestaande risicokern bruikbaar voor echte rekeningadministratie

**Bouwen**

- Haal paper-specifieke aannames uit de gemeenschappelijke risicoberekening. In `risk.rs` staat nu onder meer een vaste simulatiefee van 10 basispunten en een EUR-beperking. Behoud dit als herkenbaar simulatieprofiel, niet als exchange-fee.
- Introduceer één gedeeld ledger voor geld, beschikbare/reserveerde inventaris, orders, fills, fees, rebates, cashflows en waardering. Boek iedere mutatie exact eenmaal met Decimal en een unieke externe of interne identiteit.
- Verwerk fees in de werkelijk gerapporteerde valuta, inclusief vermindering van ontvangen basismunt. Bereken verkoopbare hoeveelheid, kostprijs en winst/verlies daarna opnieuw zonder dubbel tellen.
- Voeg actuele maker/taker-fees en herleidbare conservatieve kostenreserves toe. Een onzekere fee krijgt geen gefingeerd exact bedrag.
- Waardeer meerdere toegestane valuta met expliciete bron en dataversheid. Een ontbrekende conversie blokkeert de betreffende nieuwe blootstelling. Houd AI-kosten in USD apart totdat een geldige conversieregel is vastgelegd.
- Neem externe orders, handmatige transacties en stortingen/onttrekkingen mee. Cashflows mogen dagverlies en drawdown niet kunstmatig resetten. Voor spot zijn funding/leverage niet van toepassing; toon dat expliciet.
- Maak reserveringen, voorstellen en audit atomair. Gelijktijdige bots en handmatige orders kunnen dezelfde middelen niet dubbel besteden. Totale risico's blijven accountbreed gelden, ook bij virtuele botbudgetten.

**Acceptatie:** referentieboekingen voor quote- en basefees, partial fills, annuleringen, rebates, cashflows, meerdere bots en herstel sluiten exact aan. Dagverlies, cumulatief verlies en high-water-mark overleven herstart en dagwisseling. De bestaande paper-/replaycontroles blijven slagen.

### Stap 3. Bouw continue private accountsynchronisatie

**Bouwen**

- Vervang de korte kanaalprobe door blijvende private verbindingen voor orders, balances en posities, aangevuld met de noodzakelijke snapshots en gepagineerde historie van fills en rekeningmutaties.
- Buffer updates tijdens het ophalen van snapshots en reconcile per entiteit. Documenteer de werkelijke kanaalsemantiek; veronderstel geen gezamenlijke sequence of atomair snapshot over alle private kanalen.
- Dedupliceer op exchange-identiteiten en cumulatieve hoeveelheden. Oudere updates mogen een definitieve status niet terugdraaien. Een fill kan eerder aankomen dan een submit-antwoord.
- Detecteer externe orders en inventariswijzigingen. Koppel ze aan accountbrede risico's zonder automatisch een bot-eigenaar te verzinnen.
- Houd verbinding, heartbeat, reconnect/backoff/jitter, begrensde buffers, overlap bij historie-ophaling en periodieke vergelijkingscontroles bij. Overflow of onverklaarde verschillen veroorzaken resync en een entryblokkade.
- Toon accountgereedheid, laatste geslaagde synchronisatie, open verschillen en ontbrekende bescherming in Systeemstatus. Startup en reconnect openen entries pas na geslaagde reconstructie.

**Acceptatie:** disconnects, herstart, gemiste/duplicaat/uit-volgorde events, externe transacties en gepagineerde snapshots leiden tot dezelfde verklaarde rekeningstate. Onverklaarde verschillen blijven zichtbaar en blokkeren entries.

### Stap 4. Bouw de enige duurzame ordergateway

**Bouwen**

- Maak een afzonderlijk gatewayproces dat als enige de noodzakelijke OKX-credentials ontvangt. De paperbinary blijft zonder import van private exchange-uitvoering. UI, Jev en Pine krijgen geen gatewaysecret of vrije netwerkorderfunctie.
- Laat de vaste risicokern een beperkte, eenmalig consumeerbare uitvoeringsmachtiging vastleggen, gekoppeld aan intent, account, modus, instrument, hoeveelheid, prijsgrenzen, strategieversie, stateversie en vervaltijd. Een aangeleverd veld `approved: true` geeft nooit autoriteit.
- Leg intent, reservering en verzendopdracht duurzaam vast vóór verzending. Voer vlak vóór verzending de vaste hercontrole uit. Bij verouderde state volgt opnieuw beoordelen of weigeren, geen blind vertrouwen op een oud besluit.
- Implementeer plaatsen, annuleren, wijzigen en statusopvragen, met unieke client-id's en correlatie. Controleer zowel transportresultaat als exchange-resultaat per order.
- Werk alle OMS-statussen af: intent, pending submit, accepted, partial fill, filled, pending cancel, canceled, rejected en unknown. Een ACK verandert geen balans alsof er al een fill is.
- Houd reserveringen vast bij een onbekende uitkomst. Onderzoek exchange-status en recente fills voordat een mogelijke herverzending is toegestaan. Een tijdelijk ontbrekende zoekuitkomst is niet op zichzelf bewijs dat de eerste order nooit bestond.
- Modelleer races tussen cancel/amend en fills, inclusief wijzigingen in benodigde reserves. Maak geslaagde cancel/amend afhankelijk van bevestigde toestand.
- Implementeer requestlimieten per toepasselijke dimensie en reserveer capaciteit voor risicoreductie en reconciliatie. Sluit niet-toegestane operaties expliciet uit van de gateway.

**Acceptatie:** een crash op iedere overgang tussen reserveren, verzenden, ontvangen en opslaan veroorzaakt geen dubbele economische uitvoering of voortijdige vrijgave. Er is geen claim van exactly-once-netwerkbezorging. Ongeldige of verlopen machtigingen bereiken de exchange niet.

### Stap 5. Maak bescherming en noodbediening operationeel

**Bouwen**

- Leg per werkelijk toegestaan ordertype vast hoe entry, stop-loss en take-profit samenhangen. Bevestig native bescherming op het betreffende account; gebruik geen stilzwijgende lokale vervanging wanneer de vereiste exchange-bescherming ontbreekt.
- Controleer na iedere partial fill en fee welke inventaris beschermd moet worden. Begrens spotverkopen op beschikbare inventaris en behandel dust, minimumgrootte en afgeronde hoeveelheden expliciet.
- Werk het tijdvenster tussen fill en beschermingsbevestiging uit met meetbare timeout, blokkade van nieuwe entries, alarmering en vooraf gekozen afbouwbeleid. Beloof geen onmogelijke nulduur van dit venster.
- Vervang bescherming alleen volgens een geteste volgorde die bestaande bescherming behoudt zolang dat kan. Verifieer gevolgen van wijzigingen en reeds getriggerde algo-orders.
- Maak drie aparte acties: `PAUSE_ENTRIES`, `CANCEL_ENTRY_ORDERS` en `EMERGENCY_FLATTEN`. Toon welke entries, beschermingsorders en inventaris hierdoor geraakt worden.
- Test het werkelijke bereik van Cancel All After. Behandel dit als annulering, niet als sluiten van posities. Stel een scope vast die geen onverwachte gevolgen heeft voor bescherming of andere orders.
- Bewaar de slippagelimiet en het alternatief bij onvoldoende liquiditeit als vaste policy. De UI meldt een onvoltooide afbouw en reden; Jev kan dit beleid niet herschrijven.

**Acceptatie:** full/partial fill, fee in basismunt, cancelrace, beschermingsweigering, replacement, uitval van de app en herstel hebben ieder een geteste uitkomst. Bescherming wordt pas als actief getoond na exchangebevestiging.

**Broncontrole:** de actuele OKX-documentatie beschrijft Cancel All After als het annuleren van pending orders, met account- of tagscope. De precieze gevolgen voor de gekozen ordercombinatie moeten op het eigen account worden getest. [OKX V5](https://my.okx.com/docs-v5/en/#order-book-trading-trade-post-cancel-all-after)

### Stap 6. Dwing exclusief eigenaarschap en gescheiden modi af

**Bouwen**

- Combineer duurzame eigenaarsregistratie met afdwingbare toegang tot credentials en uitgaand orderverkeer. Alleen een databaselock is onvoldoende wanneer een oude gateway nog credentials en netwerktoegang heeft.
- Ontwerp overdracht eerst als stoppen, toegang van de oude gateway afsluiten, mogelijke lopende requests afhandelen/reconcilen, daarna de nieuwe eigenaar toelaten. Een ownershipnummer in de eigen database kan een reeds verstuurd exchange-request niet terughalen.
- Test twee lokale deployments, ook met aparte databases. Dit test dezelfde dubbele-executor-fout zonder een VPS te bouwen. Geen automatische overname zolang de oude executor niet aantoonbaar is uitgeschakeld.
- Scheid PAPER_LOCAL, OKX_DEMO en LIVE met afzonderlijke rollen, credentials, volumes/namespaces, dashboards en startprofielen. Vermijd credentialswitches in een lopend proces.
- Promoveer bots via een nieuwe deployment met gepinde strategie en opnieuw bevestigde instellingen. Een bot met open orders of exposure krijgt geen modustoggle.

**Acceptatie:** de oude of tweede executor kan na overdracht geen orders meer indienen. Bij verlies van eigenaarschap stopt verzending. Negatieve tests bewijzen dat paper nooit Demo/LIVE bereikt en een Demo-deployment geen liveconfiguratie kan gebruiken.

### Stap 7. Doorloop echte Demo-acceptatie en de volledige foutmatrix

**Volgorde**

1. Draai de volledige testexchange- en regressiesuite zonder externe credentials.
2. Voer read-only onboarding en continue synchronisatie op de bevestigde Demo-rekening uit.
3. Leg Demo-instrumenten, testbudget, verlieslimieten en testscenario's vast voordat er Demo-orders komen.
4. Doorloop handmatige order, native bot en gevalideerde Pine-bot: preview, toestemming risicokern, submit, ACK, partial/full fill, bescherming, amend, cancel en sluiten.
5. Observeer en reconcile ook extern geplaatste Demo-orders. Test herstel zonder browser en na gateway-/databaserestart.
6. Voer storingen uit in een geïsoleerde testomgeving. Koppel echte Demo-observaties aan reproduceerbare offline foutinjectie; forceer geen destructieve storing op de gebruikers-Mac.
7. Voer een voorgestelde duurproef van minimaal 72 uur uit op vastgelegde hardware en belasting. Dit is een acceptatievoorstel, geen al geplande achtergrondtaak. De proef bevat scenario's en observaties; alleen verstreken tijd telt niet als bewijs.

**Verplichte foutmatrix**

| Storing | Vereist gedrag en bewijs |
| --- | --- |
| Sequencegat, oude candle, klokafwijking | Nieuwe exposure geblokkeerd, datakwaliteit zichtbaar, resync vereist |
| Private disconnect of gemiste updates | Reconciliatie vóór hervatten, beschermingsstatus opnieuw gecontroleerd |
| Volle queue of burst | Geen stil verlies van order-/risico-events; expliciete blokkade/resync |
| Jev-timeout, laat antwoord of gewijzigde state | Geen nieuwe AI-afhankelijke exposure; vaste bescherming blijft onafhankelijk |
| Dubbele/uit-volgorde fill, partial fill | Eén correcte boeking, geen statusregressie, correcte fee/inventaris/reserve |
| Exchange accepteert, antwoord ontbreekt | UNKNOWN, reservering behouden, onderzoek vóór mogelijke retry |
| Crash vóór/na submit of vóór lokale opslag | Herstel uit duurzame intent en exchange-state; geen blinde resend |
| Verloren cancel/amend, fill tijdens cancel | Pending/unknown behouden tot bevestiging; geen fictief vrij saldo |
| Database-uitval, volle schijf | Geen onduurzame nieuwe order; vooraf gedefinieerd beschermingsbeleid en alarm |
| Rate-limit of onbereikbare exchange | Beperkte retries en gereserveerde beschermingscapaciteit, geen requeststorm |
| Twee bots claimen hetzelfde budget | Atomaire afwijzing; gezamenlijk account- en muntmaximum blijft gelden |
| Twee executors of verloren ownership | Slechts één bevoegd verzendpad; oude toegang daadwerkelijk geblokkeerd |
| Stopweigering, replacement, Cancel All After | Ontbrekende bescherming zichtbaar; vaste herstel-/afbouwactie |
| Restart na verliesstop of dagwisseling | Limieten/HWM blijven geldig; geen automatische opheffing risicostop |
| Foute Pine-code of gemanipuleerd AI-voorstel | Geen toegang tot gateway, credentials of verruiming van regels |
| Verkeerde modus/credentials/opslag | Startup of mutatie geweigerd; geen overgang naar LIVE |

**Acceptatie:** alle scenario's hebben verwacht resultaat, werkelijk resultaat en herhaalbaar bewijs. Elk onverklaard verschil in orders, inventaris, reserves of bescherming blokkeert M1. Niet op Demo reproduceerbare gevallen blijven als zodanig vermeld en worden aanvullend in de testexchange bewezen.

## 5. Open werk uit eerdere fases

### Stap 8. Maak terminalbediening en botbeheer compleet

**Bouwen**

- Volledig versleepbare/dockbare panelen, opgeslagen layouts en herstel na herladen. Maak donker/licht, leesbare dichtheid, toetsenbord, focus en statuslabels consequent.
- Rond Terminal, Screener, Pine Editor, Bot Studio, Backtests, Nieuws/Sentiment en Systeemstatus af als samenhangende werkruimtes. Controleer met echte data of alle getoonde tabellen, filters en knoppen werken.
- Toon account, modus, actieve bots, versheid, verbinding, risico en gemeten latency permanent. Geef PAPER_LOCAL, OKX_DEMO en LIVE een onmiskenbaar verschillend label.
- Verbind het orderticket en order-/positie-/SL/TP-lijnen aan echte OMS-state. Slepen geeft eerst een valide wijzigingspreview en gaat daarna via dezelfde kern. Toon aantallen, fees, risico, slippage en afwijsreden.
- Bouw een expliciet activeerbare beveiligde snelhandelsmodus met zichtbare sessiestatus, grenzen en eenvoudig uitzetten. Snelhandel slaat geen risicocontroles over.
- Maak tekeningen serverpersistent en migreer bestaande lokale tekeningen gecontroleerd. Voeg interactieve ankerbewerking toe; documenteer of een alert een vastgelegde versie of een bewust bijgewerkte lijn volgt.
- Maak multi-instrument botconfiguratie, triggers/frequentie, accountbrede exposure, kosten en afwijsredenen compleet. Versies, clone, rollback, stop/pauze en promotie blijven gecontroleerd en herleidbaar.
- Maak de eerste ingebruikname doorloopbaar: connecties controleren, modus kiezen, limieten bevestigen, preview uitvoeren, status begrijpen. Onbekende gegevens en beperkte rechten blijven zichtbaar.

**Acceptatie:** browserproeven van eerste start tot orderbeheer in paper en Demo, inclusief herladen, meerdere schermformaten, toetsenbord, fouten en verlopen sessies. Geen klikbare schijnfuncties. De browser mag dicht zonder serverbots en alerts te stoppen.

### Stap 9. Voltooi marktdata, historie en simulatie

**Bouwen**

- Business-WebSocket-candles met expliciet open/gesloten bars, gecontroleerde aanvulling na disconnect en consistente hogere timeframes. Behoud REST voor aanvulling en herstel.
- Onderzoek en implementeer werkelijk beschikbare history-trades en grotere historische downloads. Leg licentie, resolutie, tijdsbereik, ontbrekende perioden en herkomst vast vóór gebruik.
- Voeg hervatbare bulkimport en lokale opname van trades/L2 toe met begrensde opslag, schema-/sequencevalidatie, overlapdedup en zichtbare coverage. Opname vanaf vandaag is geen historische dekking voor gisteren.
- Breid candle-replay uit met afzonderlijke tick-/orderboekreplay. Modelleer echte of expliciet gekozen maker/taker-fees, partial fills, liquiditeit, slippage, vertraging, minimumorders en geschatte queuepositie.
- Respecteer tijdsvolgorde: gesloten candle pas daarna verhandelbaar, geen toekomstdata, geen Heikin-Ashi-uitvoering als marktprijs, conservatieve volgorde bij onduidelijke stop/target.
- Maak datasets en rapporten reproduceerbaar met versie/hash, brondekking, parameters, kostenprofiel en onzekerheden. Verifieer nettoresultaat, drawdown, turnover, exposure en train/OOS-scheiding voor native én ondersteunde Pine-routes.
- Controleer de tien native indicators met rechtmatig beschikbare referentiereeksen, warm-up/`na`, stijlen/eigen panelen en expliciete VWAP-sessie-/tijdzonegrenzen.

**Acceptatie:** dezelfde invoer levert dezelfde berekening en boeking. Datagaten blijven zichtbaar. Candle- en orderboekresultaten zijn herkenbaar verschillende simulaties en worden nooit als exchangefills gepresenteerd.

### Stap 10. Breid Pine uit in toetsbare profielen

Breid de bestaande runtime-adapter uit, zonder volledige TradingView-compatibiliteit te beloven:

| Profiel | Af te bouwen | Vrijgavebewijs |
| --- | --- | --- |
| Closed-bar uitgebreid | TA, control flow, collecties, types, waarschuwingen, plots en alerts binnen vastgelegde dekking | Rechtmatige v5/v6-referenties, numerieke toleranties, warm-up en concrete foutdiagnostiek |
| Multi-timeframe | `request.security`, tijdsuitlijning, gaps/lookahead en beperkte door de host aangeleverde data | Geen toekomstlek, correcte handelskalender/tijdzone en verschil tussen voorlopige/definitieve waarden |
| Realtime | Intrabar, rollback, `var`/`varip`, barstatus en opnieuw berekenen | Vastgelegde tickreeksen inclusief reload/restart; repainting duidelijk zichtbaar |
| Strategy | Ondersteunde `strategy.*`-calls naar centrale order-intents; status terug naar script | Geen eigen uitvoeringsautoriteit; dezelfde risicokern en verklaarde emulatorafwijkingen |
| Visueel | Pine-tekenobjecten, stijlen en lifecycle | Begrensde objectaantallen, correcte rollback/opruiming en gelijke tijd-/prijsankers |

- Werk `PINE_COMPATIBILITY.md` per profiel bij, inclusief onbekende functies en blijvende uitsluitingen. Runtimeversie alleen wijzigen na licentie- en regressiecontrole.
- Voer dezelfde ondersteunde semantiek uit in chart, backtest, paper, Demo en LIVE. Een profiel zonder geaccepteerde handelssemantiek krijgt geen handelsvrijgave.
- Houd netwerk, hostfiles, credentials en ongecontroleerde imports buiten scripts. Uitgebreide taalondersteuning verruimt de sandbox niet automatisch.
- Voeg adversarial tests toe voor eindeloze berekening, uitvoer/geheugen, malformed bron en ontsnappingspogingen. Deadlines en foutmeldingen moeten via de echte workerroute worden bewezen.

**Acceptatie:** ieder aangeboden profiel heeft een gepind corpus en reproduceerbare vergelijking. Niet-ondersteunde functies geven een concrete compilefout. Protected/invite-only bron wordt niet omzeild.

**Broncontrole:** Pine maakt onderscheid tussen historische uitvoering en realtime herberekening met rollback; `varip` heeft daarbij andere persistentie. Daarom worden realtime en multi-timeframe apart vrijgegeven. [TradingView execution model](https://www.tradingview.com/pine-script-docs/language/execution-model/)

### Stap 11. Rond Jev, nieuws en vergelijking af

**Bouwen en accepteren**

- Controleer het actuele Decisions-contract en de gepinde modelversie. Behoud een providerinterface voor eventueel later direct TypeSafe, zonder automatische model-/providerwissel.
- Test met een nieuwe lokaal ingevoerde OpenRouter-key en vooraf bevestigd USD-kostenbudget de werkelijk ontvangen antwoorden, confidence/probabilities, kosten en fouten. Bewaar alleen noodzakelijke niet-geheime replay-invoer.
- Verifieer vóór én na een modelcall account-/snapshot-/strategie-identiteit, TTL en vaste risicolimieten. Test kosten bij timeout, fout en laat antwoord; onbekende kosten mogen budget niet ongemerkt vrijmaken.
- Maak de keten zichtbaar: aanleiding → compacte state → ontvangen voorstel → vaste risicobeoordeling → toegestane of afgewezen intent. Maak geen redeneertekst die het model niet heeft geretourneerd.
- Rond CryptoPanic-onderzoek af met plan, rechten, schema, quotum en dekking. Bouw de adapter pas voor een aantoonbaar toegestane route. Voeg CoinGecko-metadata/marktcontext toe onder dezelfde voorwaarden.
- Maak cross-provider deduplicatie, assetkoppeling, publicatietijd versus eerste ontvangst, TTL en eventuele modelsentimentlabels compleet. Houd OKX-status en dagelijkse Bitcoin Fear & Greed eerlijk afgebakend.
- Test HTML/script-injectie, prompt injection en SSRF via providerdata. Externe nieuwsinhoud krijgt geen instructie- of uitvoeringsautoriteit.
- Voer een vooraf vastgelegde forward-papervergelijking uit van dezelfde strategie met en zonder Jev. Leg start, duur, dataset, botinstellingen, risicolimieten en kostenbudget vast vóór de proef. Beide armen krijgen vergelijkbare onafhankelijke simulatieomstandigheden; gedeelde account-/liquiditeitsinvloed wordt voorkomen of als beperking gerapporteerd.

**Acceptatie:** echte calls en werkelijke kosten zijn gecontroleerd; storing van Jev maakt vaste bescherming niet afhankelijk van AI. Vergelijkingsrapporten bevatten nettoresultaat, drawdown, exposure, fees, AI-kosten en onzekerheid. Een korte proef geldt als functionele acceptatie, niet als bewijs van winstgevendheid of voorspellend vermogen.

**Broncontrole:** OpenRouter documenteert Jev via Decisions en System One. De bestaande integratie gebruikt Decisions; dit plan introduceert geen gewone chatcompletion als vervanger. [OpenRouter Jev](https://openrouter.ai/docs/guides/community/jev)

### Stap 12. Lever de expliciet gevraagde aanvullende analyse

- **Volume profile:** kies en vermeld of volume uit echte trades of een benaderde candleverdeling komt. Een benadering krijgt dat label. Test prijsbakken, sessies en volumeconservering.
- **Orderflow:** gebruik alleen werkelijk beschikbare trade-/aggressorinformatie, met correcte eenheden en ontbrekend-beleid. Leidt de bron de agressor niet betrouwbaar af, toon geen verzonnen koop-/verkoopdruk.
- **Heatmaps:** gebruik opgeslagen L2-snapshots/delta's met geldige continuïteit en zichtbaar diepte-/tijdvenster. Toon gaten; presenteer beperkte zichtbare diepte niet als het volledige orderboek.
- **DeMARK/Sequential/Combo:** stel eerst toegestane regels, licentie en rechtmatige referenties vast. Implementeer vervolgens de toegestane variant met bron, afwijkingen en tests. Een zelfstandig gespecificeerde variant vergt een expliciete keuze en krijgt geen misleidend officieel label.

**Acceptatie:** brondata, rekenregels en referentiecases zijn aantoonbaar. Deze punten blijven in de scope; ontbrekende rechten of data worden als concrete blokkade geregistreerd, niet als afgeronde knop. BUILD_SPEC.md plant volume profile/orderflow/heatmaps expliciet als vervolgstappen.

## 6. Operatie en eindoplevering

### Stap 13. Maak lokale operatie en volledig herstel betrouwbaar

**Bouwen**

- Lever duidelijke lokale Compose-profielen voor ontwikkeling, paper, Demo, productie en optionele observability. De productiekern werkt zonder frontend. Publiceer alleen de bedoelde localhost-ingang.
- Rond gatewaybeveiliging, rolverdeling, minimale rechten, credentialrotatie, sleutelherstel en auditing af. Nieuwe secrets blijven in niet-synchroniserende lokale opslag of ondersteunde lokale sleutelhanger, nooit in iCloud, Git, logs of images.
- Behoud sessieverval, WebSocket-origincontrole, CSRF/CSP, invoerbegrenzing en rate limiting. Leg vast welke bevestigingen nodig zijn voor gevoelige configuratie, snelhandel en vrijgave.
- Maak versleutelde streaming-backups die grotere datasets dan de huidige 256 MiB-grens aankunnen. Neem database, Parquet, servertekeningen, workspaces, configuratie, deployments en risicostatus mee. Houd credential- en backupsleutels apart met herstelprocedure.
- Bouw configureerbare backupplanning, retentie, integriteitscontrole en foutmelding. Stel als voorstel dagelijkse backups plus een backup vóór iedere migratie voor; bestemming, bewaartermijn en gegevensverlies-/hersteltijddoelen worden vóór activering vastgelegd.
- Test herstel naar een lege lokale database en een vervangende lokale omgeving. Verifieer ook grote backups, ontbrekende/verkeerde sleutel, beschadigde backup en onvolledige dataset. Herstel activeert nooit automatisch handel.
- Maak upgrade-/rollbackprocedures met expliciet beleid voor open orders, beschermingsorders, posities en schemawijzigingen. Geen blind automatisch herstarten.
- Maak alarmen voor disconnect, UNKNOWN, ontbrekende bescherming, drawdown, foutpercentages, disk en backupfalen compleet. Bewijs serverregistratie zonder browser. Een extern notificatiekanaal vergt een afzonderlijke keuze en inrichting.
- Documenteer Mac-slaap, stroom-/netwerkuitval en Dockerstop. Lokale 24/7-uitvoering vereist een geschikte altijd beschikbare lokale host. Verifieer wat op de exchange beschermd blijft wanneer de Mac wegvalt.

**Acceptatie:** een lege omgeving kan met de gedocumenteerde benodigdheden worden hersteld, waarna eerst reconciliatie en menselijke controle volgen. Geen ongemerkte bots, reset van verlieslimieten of duplicaatorders. Gemeten herstelduur en dataverlies passen binnen de vooraf gekozen doelen.

**Scopegrens:** dit blijft lokaal. VPN/TLS en MFA/passkeys voor remote toegang worden pas een uitvoeringspakket bij een afzonderlijk verzoek om remote gebruik; dit plan geeft geen toestemming om de app te publiceren.

### Stap 14. Voer complete releaseacceptatie uit en maak LIVE technisch gereed

**Bouwen en controleren**

- Bouw het afzonderlijke LIVE-profiel en de live-uitvoeringsadapter op dezelfde geaccepteerde kern. Gebruik geen gedeelde Demo-state, automatische fallback of standaard ingevuld livebudget. Afwezige vrijgave betekent technisch geen verzending.
- Verifieer regionale LIVE-capabilities afzonderlijk; Demo-ondersteuning bewijst niet automatisch gelijk gedrag of gelijke rechten in LIVE. De eerste live-accountcontrole is read-only.
- Voer alle bestaande regressies en nieuwe scenario's uit op de definitieve bron-/imageversie. Test werkelijk Linux arm64 en amd64. Vermeld of amd64 onder emulatie of op eigen hardware draaide.
- Instrumenteer ontvangst, features, Pine, Jev, risk, duurzame registratie, submit, ACK en fill afzonderlijk en volledig. Publiceer p50/p95/p99, fouten, hardware, events/s, instrumenten, actieve bots en indicatorlast.
- Toets het doel uit de bouwprompt: p95 onder 10 ms voor een eenvoudige lokale native event→gevalideerde-intentroute, exclusief AI/netwerk en duurzame registratie. Meet daarnaast de volledige route mét duurzaamheid; het eerste getal vervangt de echte orderlatency niet.
- Leg de ondersteunde capaciteitsgrens vast. Test normale belasting, een reproduceerbare burst en de duurproef met actieve bots. Meet geheugen, queues, UI-vertraging en herstel. Vertraag UI waar nodig, verlies geen kritieke events.
- Herhaal dependency-/imagescans op de releasekandidaat, lokaal en zonder project-/imagemetadata naar een clouddienst te sturen. Laat een afzonderlijke securityreview de gateway, credentials, Pine-sandbox, browser en herstel beoordelen. Geen onopgeloste releaseblokkerende bevindingen.
- Werk alle documenten, migraties, lockfiles, Dockerfiles, `.env.example`, installatie-/herstelhandleiding, capabilities, providerrechten, Pine-matrix en benchmarks bij. Leg ook negatieve resultaten en niet-geteste scenario's vast.
- Loop de volledige gebruikersreis door: installeren, starten, account verbinden, analyseren, script gebruiken, bot maken, evalueren, Demo-order beheren, pauzeren, herstellen en gecontroleerd promoveren.

**Acceptatie:** M1 en M2 zijn gehaald; vereiste tests, herstel en review slagen voor exact dezelfde release. Alle zestien specsecties hebben een verklaring met bewijs. Als een gewenste integratie door rechten/data niet kan worden geleverd, blijft de volledige oplevering gedeeltelijk geblokkeerd totdat Willem de beperking of een alternatief bewust kiest.

**Oplevering:** lokaal releasepakket met commit-/image-identiteiten, migraties, test-/benchmarkrapport, herstelrapport, securitybevindingen en een afvinkbare vrijgavelijst. Geen fictief nieuw testtotaal vooraf bepalen; toets scenario's en invarianten.

### Stap 15. Rond persoonlijke inrichting en vrijgave af

1. Laat Willem in de lokale app account en modus, toegestane instrumenten, handelsbudget en alle bot-/accountlimieten bevestigen. Laat ontbrekende waarden niet stilzwijgend op een bruikbaar livebedrag vallen.
2. Configureer nieuwe credentials veilig lokaal, met alleen benodigde rechten, zonder withdrawals en met toepasselijke toegangsbeperking. Bewaar de bevestigde capabilitystatus.
3. Laat eerst de gekozen paper-/Demo-inrichting doorlopen en toon concrete resultaten en beperkingen.
4. Maak een concrete livevrijgave met account, instrumenten, maximumbudget, verliezen, drawdown, exposure, orders, slippage, versheid, AI-budget en gebruikte strategieversies. Willem bevestigt die expliciet pas na M3.
5. Willem plaatst een eventuele eerste echte order via de terminal, binnen de expliciet bevestigde grenzen. Houd deze eerste livecontrole begrensd en begeleid; bevestig echte fills, fees, bescherming en accountstate. LIVE-gedrag dat alleen offline/Demo is getest krijgt pas daarna echt livebewijs.
6. Hervat of verruim daarna niets automatisch. Een risicostop vraagt menselijke controle; Jev kan geen grens aanpassen of zichzelf toestemming geven.

**Acceptatie:** de app is technisch opgeleverd, de persoonlijke instellingen zijn aantoonbaar bevestigd en de feitelijk geactiveerde omgeving is correct geregistreerd. De app kan technisch af zijn terwijl Willem bewust alleen paper/Demo gebruikt.

## 7. Wat er wanneer van Willem nodig is

| Invoer/keuze | Nodig voor | Blokkeert niet |
| --- | --- | --- |
| Nieuwe OKX Demo-key, secret en passphrase via lokale invoer | Read-only accountacceptatie in 1/3 en echte Demo-tests in 7 | Kern, gateway, UI en offline fouttests bouwen |
| Bevestigde accountregio, spotinstrumenten en testlimieten | Concrete Demo-acceptatie | Publieke data en generieke modulebouw |
| Nieuwe OpenRouter-key en bevestigd USD-kostenbudget | Echte Jev-tests en forwardproef in 11 | Adapter, fixtures, kostenregels en UI |
| Providerplan/rechten en eventueel noodzakelijke sleutels | CryptoPanic/CoinGecko en historische datasets | Andere providers en overige functionaliteit |
| Toegestane DeMARK-specificatie/licentie of bewuste keuze voor een eigen variant | Dat specifieke deel van 12 | Fase 5 en overige analysefuncties |
| Rechtmatig beschikbare eigen Pine-scripts, indien gewenst | Extra acceptatie voor Willems concrete scripts | Een openbaar, rechtmatig testcorpus bouwen |
| Lokale backupbestemming, retentie en gewenste beschikbaarheid | Automatische backup-/herstelafspraken in 13 | Backupmechanisme en geïsoleerde hersteltests |
| Live-account, instrumenten, budget en alle limieten plus expliciete vrijgave | Alleen activering van LIVE in 15 | LIVE-adapter technisch gereed maken en alle onafhankelijke ontwikkeling |

Keys worden nooit via chat gevraagd. Het ontbreken van deze invoer moet steeds als specifieke acceptatieblokkade worden vermeld, niet als reden om al het bouwwerk te stoppen. Dit plan stelt geen kalenderdeadline, persoonlijk handelsbudget of betaalde abonnementskeuze vast.

## 8. Traceerbaarheid naar de volledige bouwprompt

| Hoofdstuk BUILD_SPEC.md | Afgedekt door stappen |
| --- | --- |
| 1. Doel en uitgangspunten | 1–2, 6, 14–15; lokaal, spot, vaste kern |
| 2. Architectuur en stack | 1–4, 6, 10, 13–14 |
| 3. Terminal en bediening | 8, 14–15 |
| 4. OKX/accountdetectie | 1, 3–7, 14–15 |
| 5. Marktdata en historie | 3, 7, 9, 14 |
| 6. Charts/indicators/tekenen/alerts | 8–9, 12–13 |
| 7. Pine | 8, 10, 14 |
| 8. Bot Studio | 2, 6–8, 10–11 |
| 9. Jev | 2, 7, 11 |
| 10. Risicobeheer | 1–2, 4–7, 15 |
| 11. OMS en herstel | 3–7, 13–15 |
| 12. Paper/Demo/backtesting | 2, 6–7, 9–11, 14 |
| 13. Nieuws en sentiment | 11 |
| 14. Snelheid | 3–4, 7, 9–10, 14 |
| 15. Beveiliging en operatie | 4–7, 10, 13–15; geen clouddeployment |
| 16. Tests en oplevering | 1, 7, 13–15 |

## 9. Eerstvolgende bouwopdracht

Begin bij **stap 1 en daarna stap 2**: leg de huidige lokale bron vast, definieer het account-/ordercontract en bouw de gedeelde rekeningadministratie plus testexchange. Ontwerp de gatewayisolatie en exclusieve toegang nu mee, zodat deze later geen losse noodoplossing worden.

Het eerste concrete opleverpunt is een reproduceerbare test waarin twee bots hetzelfde saldo proberen te reserveren, de testexchange een gedeeltelijke fill met fee in basismunt meldt en vervolgens een timeout plus herstart optreedt. De risicokern moet daarna één verklaarbare rekening en orderstatus reconstrueren, zonder dubbele uitvoering of gereset verliesbudget. Dit kan worden gebouwd zonder persoonlijke keys of echt handelsgeld.
