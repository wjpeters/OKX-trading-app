# OKX Bot Training Lab met neuro-evolutie

## 1. Doel en gekozen uitgangspunten

Breid de bestaande lokale OKX-terminal uit met een trainingsomgeving die bots ontwikkelt, vergelijkt en kwalificeert voor live handelen.

Jouw keuzes:

- **EUR- en USDC-spotmarkten**, zonder leverage of shortposities.
- Afzonderlijke bots voor **1m, 5m en 15m**.
- **Laya lokaal** als AI-component.
- Zelfstandige keuze uit een vaste strategiebibliotheek.
- Doorlopend trainen binnen ingestelde resourcegrenzen.
- Maximaal **5% drawdown tijdens kwalificatietests** en minimaal **30 dagen forward-paper**.
- **Jij geeft iedere botversie vrij voor live handelen.**

De trainer zoekt naar aantoonbaar bruikbare parameters. Hij kan ook eindigen met **geen geschikte bot gevonden**. Meer trainen garandeert geen winstgevendheid.

Alles blijft binnen het bestaande lokale project, buiten iCloud. De huidige terminal, Bot Studio en ordergateway vormen de basis.

## 2. Handelsmodel en trainingsmethode

### Drie gescheiden onderdelen

| Onderdeel | Verantwoordelijkheid |
|---|---|
| Laya | Beoordeelt de marktsituatie op basis van beschikbare marktkenmerken. |
| Neuraal strategiemodel | Kiest een strategie of niet handelen; de gewichten en strategieparameters worden geëvolueerd. |
| Vaste risicokern | Bepaalt of een voorstel uitvoerbaar is en begrenst bedrag, exposure en verlies. |

Laya ondersteunt lokale uitvoering en domeinspecifieke training. Voor deze eerste versie gebruiken we een vastgezette checkpoint; de neuro-evolutie traint het kleinere strategiemodel eromheen. Zo blijven experimenten reproduceerbaar en betaalbaar. [Laya-project](https://github.com/NandhaKishorM/laya)

De bestaande Jev-koppeling blijft beschikbaar voor een latere vergelijking. Er komen geen betaalde Jev-aanroepen in de standaardtrainingsroute.

### Strategiebibliotheek

De eerste versie bevat:

- **Trend:** EMA-kruising met trendfilter.
- **Breakout:** uitbraak boven een prijsbereik met volatiliteits- en volumefilter.
- **Mean reversion:** Bollinger Bands en RSI.
- **Niet handelen:** wachten wanneer geen strategie voldoende voordeel biedt.

Het model kiest bij iedere gesloten candle. Een geopende positie behoudt haar oorspronkelijke exitregels. Een andere strategiekeuze mag een bestaande stop niet verruimen. Bescherming loopt onafhankelijk van AI en candlebeslissingen.

### Neuro-evolutie

Gebruik een klein netwerk met één verborgen laag van 16 neuronen. Inputs bestaan uit rendementen, volatiliteit, trend, momentum, volume, positiegegevens, geschatte kosten en Laya-regimebeoordelingen. Outputs zijn de drie strategieën en niet handelen.

Evolueer:

- Netwerkgewichten en biases.
- Indicatorperioden en signaaldrempels.
- Stopafstand, winstdoel, maximale houdduur en cooldown binnen vooraf vastgelegde onderzoeksgrenzen.

Handelsbudgetten en harde rekeninglimieten vallen buiten de evolutie.

Gebruik **CMA-ES via pycma** voor de optimalisatie van gewichten en parameters. [Officiële implementatie](https://github.com/CMA-ES/pycma)

Startinstellingen:

- 64 kandidaten per generatie.
- Drie reproduceerbare random seeds.
- Maximaal 200 generaties per seed.
- Een ronde stopt eerder na 30 generaties zonder verbetering.
- Maximaal acht uur training per dag, twee evaluatieworkers en 4 GB trainingsgeheugen.
- Checkpoints na iedere generatie; hervatten na afsluiten of herstart.

De fitness maximaliseert het mediane nettoresultaat over trainingsblokken, met voorrang voor lagere drawdown bij gelijke resultaten. Overschrijding van risicogrenzen maakt een kandidaat ongeldig. Kosten worden volledig verwerkt; handelsfrequentie en winrate zijn geen zelfstandige optimalisatiedoelen.

Na uitputting van de zoekruimte wacht de trainer op nieuwe data. Hij verlaagt geen kwaliteitseisen om alsnog een winnaar te produceren.

## 3. Marktdata, kosten en kwalificatie

### Eerst de bestaande backtestbasis uitbreiden

De huidige botroute gebruikt 1m-candles, maximaal 5.000 candles per grafiekbacktest en vaste fees. Dit wordt vervangen door:

- Historische datasets op datumselectie, zonder stille afkapgrens.
- Gesloten 1m-candles met consistente aggregatie naar 5m en 15m.
- Dezelfde indicatoropbouw, warm-up en strategiecode in training, replay en forward-uitvoering.
- WebSocket-candles met herstel na verbindingsverlies.
- Opslag van actuele trades en orderboekdata voor latere uitvoeringsreplay.
- Datasetversies, bronvermelding, coverage en expliciete datagaten.

Begin met BTC-EUR en ETH-EUR als onderzoeksprofiel. Andere EUR/USDC-markten zijn selecteerbaar zodra voldoende historie beschikbaar is. Livehandel vereist daarnaast aantoonbare accounttoegang.

### Roundtripkosten

Iedere beoordeling rekent met:

**Nettoresultaat = verkoopopbrengst − aankoopkosten − handelsfees − overige toerekenbare kosten.**

Daarbij horen:

- Fees bij zowel instappen als uitstappen.
- Spread, slippage en marktimpact in de gesimuleerde uitvoeringsprijzen.
- Partial fills, latency, minimumorders en prijs-/hoeveelheidsafronding.
- Werkelijke feevaluta en eventuele rebates.
- Benodigde valutawaardering voor gezamenlijke EUR-risicocontroles.
- AI-kosten afzonderlijk zichtbaar; lokale rekenkosten apart in het onderzoeksrapport.

Spread en slippage worden niet nogmaals afgetrokken als ze al in de fillprijs zitten.

Lees actuele maker-/taker-fees per instrument uit de OKX-accountkoppeling. Historische aannames worden afzonderlijk vastgelegd; actuele fees gelden niet automatisch als historische waarheid. [OKX-fee-API](https://app.okx.com/docs-v5/en/#trading-account-rest-api-get-fee-rates)

De eerste livegeschikte botversie gebruikt direct uitvoerbare orders met een prijsgrens, gemodelleerd met taker-fees. Makerstrategieën volgen pas wanneer wachtrijpositie en niet-uitgevoerde orders voldoende kunnen worden getest.

### Scheiding tussen leren en bewijzen

Gebruik standaard minimaal twaalf maanden historie:

1. Vier verschuivende vensters met ongeveer 180 dagen training en 30 dagen validatie.
2. Alle normalisatie en parametertraining gebeuren uitsluitend binnen het trainingsdeel.
3. Selecteer één kandidaat per markt/timeframe op validatieresultaten.
4. Train de gekozen configuratie opnieuw op de ontwikkelperiode en zet haar vast.
5. Gebruik de laatste 60 dagen eenmaal als afgesloten eindtest.

Overlappende posities en trainingslabels mogen niet over de splits heen lekken. Een bekeken eindtest wordt nooit opnieuw als ongezien bewijs gebruikt. Na afwijzing is nieuwe, ongebruikte data nodig voor herkwalificatie.

Vergelijk steeds met dezelfde vaste strategieën zonder neuraal model, een variant zonder Laya, cash en buy-and-hold. Toon nettoresultaat, drawdown, exposure en onzekerheid naast elkaar.

### Voorgesteld kwalificatieprofiel

Deze aanvullende standaardwaarden worden vóór iedere campagne vastgezet:

| Controle | Vereiste |
|---|---|
| Afgesloten historische eindtest | Positief nettoresultaat, profit factor ≥ 1,20 en minimaal 200 gesloten roundtrips |
| Drawdown | Maximaal 5%, inclusief open posities en geschatte sluitkosten |
| Statistische toets | Positief gemiddeld netto dagrendement met blokbootstrap; Holm-correctie voor alle kandidaten in dezelfde kwalificatieronde |
| Kostenstress | Nettoresultaat blijft positief bij 1,5× totale uitvoeringskosten |
| Forward-paper | Minimaal 30 dagen én 100 gesloten roundtrips, positief nettoresultaat en maximaal 5% drawdown |
| OKX Demo | Orderverwerking, bescherming, administratie en herstel aantoonbaar correct |

Te weinig trades betekent **onvoldoende bewijs**, geen automatische goedkeuring. Paper gebruikt actuele marktdata; OKX Demo dient vooral om de technische orderroute te toetsen. Historische candles alleen kwalificeren geen snelle bot voor livegebruik.

## 4. Integratie in de terminal en livegang

### Lokale architectuur en interfaces

Behoud Rust voor marktdata, strategie-uitvoering, risico en orders. Voeg een geïsoleerde Python-trainer en een lokale Laya-service toe. De trainer gebruikt een Rust-evaluator die dezelfde strategiekern uitvoert als de terminal. Geen trainingsproces krijgt exchangecredentials.

Voeg toe:

- `TrainingCampaign`: markten, timeframes, datasets, zoekruimte, kostenprofiel en resourcegrenzen.
- `CandidateArtifact`: gewichten, parameters, featureversie, Laya-checkpoint, hashes en resultaten.
- `QualificationReport`: historische tests, forwardbewijs, kostenstress en afwijsredenen.
- `BotRelease`: exact kandidaatartifact, account, markt, timeframe, budget en bevestigde limieten.
- API-routes onder `/api/training` voor starten, pauzeren, hervatten en resultaten; afzonderlijke routes voor kwalificatie en persoonlijke vrijgave.
- Een expliciet timeframe in botconfiguraties. Bestaande bots behouden standaard 1m en hun huidige versie.

Datasets blijven in Parquet; campagnes, resultaten en auditgegevens komen in PostgreSQL. Artefacten zijn onveranderlijk en lokaal opgeslagen.

### Bediening

Voeg binnen Bot Studio een **Training Lab** toe:

**Instellen → Trainen → Vergelijken → Kwalificeren → Vrijgeven → Volgen**

Toon per kandidaat:

- Markt, timeframe en actuele strategiekeuze.
- Voortgang, generatie en gebruikte rekentijd.
- Nettoresultaat tegenover kosten, drawdown en benchmarks.
- Welke criteria slagen of falen.
- Waarom de bot koopt, verkoopt of wacht.
- Welke exacte versie paper, Demo of live draait.

### Live-uitvoering

Volg deze statussen:

**Onderzoek → Historisch geslaagd → Forward-paper geslaagd → Demo geaccepteerd → Wacht op vrijgave → Live**

Voor live-bots moeten de bestaande paper- en native orderadministratie gezamenlijk rekeninggeld en inventaris bewaken. Handmatige orders en meerdere bots mogen hetzelfde beschikbare saldo niet dubbel gebruiken.

Maak daarnaast af:

- Continue reconciliatie met OKX.
- Idempotente orderverwerking en herstel van onbekende orderstatussen.
- Bescherming van gedeeltelijk uitgevoerde posities.
- Eén actieve uitvoerder met bescherming tegen dubbele processen.
- Afzonderlijke botvrijgave; bestaande handmatige toestemming geeft geen bottoestemming.

Jij kiest het livebudget en de limieten. Nieuwe modelversies blijven eerst kandidaat. Een lopende liveversie leert niet ter plaatse bij en wordt niet automatisch vervangen.

Bij verouderde data, ontbrekende bescherming, administratieve verschillen of verlieslimieten stoppen nieuwe entries. Bestaande bescherming en herstel blijven actief. Hervatten na een risicostop vereist jouw actie.

## 5. Bouwvolgorde en acceptatietests

1. **Databasis en rekengelijkheid:** meerdere timeframes, grote datasets, uniforme indicatorstate en kostenboekhouding.
2. **Trainingskern:** strategiebibliotheek, netwerk, CMA-ES, checkpoints en resourcebewaking.
3. **Laya en Training Lab:** lokale inference, reproduceerbare modelinputs en vergelijking met de variant zonder Laya.
4. **Kwalificatie:** afgesloten eindtests, kostenstress, forward-paper en toetsbare rapporten.
5. **Botuitvoering op OKX:** gezamenlijke risicoadministratie, Demo-acceptatie en persoonlijke livevrijgave.

Verplichte controles:

- Geen toekomstige candles, labels of normalisatiegegevens in beslissingen.
- Identieke signalen bij dezelfde data en artefacten in replay en forward.
- Correcte kosten bij winst, verlies, partial fills, basefees en rebates.
- Geen onbeperkte nieuwe pogingen op dezelfde afgesloten eindtest.
- Geen overschrijding van CPU-, geheugen-, tijd- of handelslimieten.
- Geen dubbele orders na timeout, reconnect, crash of herstart.
- Geen liveorder zonder geldige vrijgave voor de exacte botversie.
- Laya-uitval blokkeert afhankelijke entries; bescherming blijft werken.
- Latencydoel onder trainingsbelasting: p95 onder 100 ms voor de lokale strategiekern en onder twee seconden van ontvangen candle tot duurzaam ordervoorstel. Exchange-ACK en filltijd worden apart gemeten.

De oplevering is geslaagd wanneer deze keten aantoonbaar werkt, inclusief het terecht afwijzen van slechte bots. Het vinden van een winstgevende bot blijft een onderzoeksuitkomst.
