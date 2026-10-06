# OKX Tradingterminal

Nieuw lokaal project volgens [BUILD_SPEC.md](BUILD_SPEC.md), gestart op 2 oktober 2026. Broncode, gegevens en sleutels blijven in `/Users/wp/WPOS-local/projects/okx-tradingterminal`, buiten iCloud. Geen hergebruik van eerdere tradingapps en geen clouddeployment.

## Starten

1. Start Docker Desktop.
2. Dubbelklik **Start OKX Terminal.command**.
3. Open http://127.0.0.1:4780.

De app start als **PAPER_LOCAL**. Je bevestigt zelf een virtuele EUR-rekening en de rekening-/botlimieten in Bot Studio → Instellingen → Rekening, posities en orders beheren. Er is geen gebruikersrekening of gebruikersbot door de ontwikkeltests geactiveerd. Jev mag voorstellen geven; alleen de vaste risicokern kan deze na hercontrole en duurzame reservering omzetten in paperorders.

```sh
rtk proxy python3 scripts/start.py
```

Stoppen via **Stop OKX Terminal.command**. Gebruik geen `down -v`: dat verwijdert de database. Browser sluiten laat bots en alerts doorlopen; Mac-slaap of Dockerstop onderbreekt lokale bescherming.

## Beschikbare werkruimtes

- Terminal: echte OKX EEA-prijzen, orderboek met sequencecontrole, tape, één tot vier gekoppelde charts, tien configureerbare indicatoren, tekeningen en opgeslagen workspaces.
- Pine Editor: eigen indicatoren en entry/exit-strategieën, conceptherstel, inputs, versies en direct opslaan naar de terminalchart of een botnode. Het expliciet begrensde closed-bar-profiel staat in [PINE_COMPATIBILITY.md](PINE_COMPATIBILITY.md).
- Bot Studio: bewerkbare nodegrafiek en canonieke configuratie, templates en versiebeheer. Een gepinde strategie kan aan een gepauzeerde paperbot zonder open exposure worden gekoppeld. Starten is een afzonderlijke actie.
- Alerts: prijs, indicator, vastgelegde tekenlijn, bot en risicostatus; servercontrole en duurzame lokale inbox.
- Backtests: vanuit Bot Studio op de werkelijk opgeslagen nodegrafiek en botlimieten. Losse Paper-/Backtestpagina’s zijn uit het menu; bestaande onderzoeks-API’s en gegevens blijven behouden. Rekening, posities en orders beheer je onder de botinstellingen.
- Nieuws: OKX announcements/status, dagelijkse Bitcoin Fear & Greed, bronstatus, Jev-kosten en ontvangen voorstellen. Ontbrekende gegevens blijven zichtbaar als ontbrekend.
- Systeem: API-sleutels voor OKX Demo/Live, Jev direct en Jev via OpenRouter, providerkeuze en AI-kostenbudget; aparte tab voor verbindingen, latency, opslag en bescherming. Zie [Systeem-instellingen](docs/SYSTEM_SETTINGS.md).

## Tijdframes

De grafiek heeft veertien tijdframes, van 1 minuut tot 3 maanden. Week (**1W**) en maand (**1M**) staan direct in de knoppenbalk; alle intervallen vind je in **Alle tijdframes**. Zie de [tijdframehandleiding](docs/TIMEFRAMES.md).

## Tekenen op de grafiek

Kies links een pictogram of open **Tekentools** voor uitleg. Klik de aangegeven punten aan en sleep daarna de vorm of de ronde punten om te bewerken. Esc annuleert; ⌘/Ctrl Z maakt ongedaan. Zie de [tekenhandleiding](docs/DRAWING_TOOLS.md).

## Externe aansluitingen

**Systeem → Aansluitingen & AI** beheert de API-sleutels voor **OKX Demo/Live**, **Jev AI direct (TypeSafe)** en **Jev AI via OpenRouter**. Kies welke Jev gebruikt moet worden en bevestig daarna zelf je AI-kostenbudget. Keys blijven lokaal versleuteld; opslaan start geen providerproef of handel. De oude CLI-setup is alleen voor offline gebruik. Zie [gebruik en opslag](docs/SYSTEM_SETTINGS.md).

De afzonderlijke Demo-onboardinghelper kan de opgeslagen Demo-key gebruiken voor accountdetectie en een private-kanaalprobe. Dit activeert geen orders. **De volledige Demo/LIVE-uitvoering is nog niet af**: continue private reconciliatie, de duurzame exchange-orderadapter, native bescherming en fencing moeten nog worden gebouwd/geaccepteerd. Zie [STATUS.md](STATUS.md) en [resterend werk](docs/REMAINING.md). LIVE kan niet worden aangezet in deze release.

**Beveilig terminal.command** stelt optionele lokale wachtwoordbeveiliging in. Backups maak/test je met `rtk proxy python3 scripts/backup.py test`.

## Handleidingen en bewijs

**Vervolgplan:** [Fase 5 en volledige afronding](docs/PHASE5_EN_AFRONDING.md), opgesteld op 4 oktober 2026. Bouwvolgorde, afhankelijkheden, acceptatiecriteria en benodigde persoonlijke invoer. Dit plan activeert geen handel.

[Gebruik van Pine/Bot Studio/Jev](docs/PHASE3_4.md) · [Risico en paper](docs/PHASE2.md) · [Architectuur](docs/ARCHITECTURE.md) · [Capabilities](docs/CAPABILITIES.md) · [API](docs/API.md) · [Operatie/herstel](docs/OPERATIONS.md) · [Threat model](docs/THREAT_MODEL.md) · [Acceptatie](docs/ACCEPTANCE.md) · [Providers/licenties](docs/PROVIDERS_LICENSES.md) · [Metingen](docs/BENCHMARKS.md).

Exacte dependencies staan in lockfiles; basisimages op digest. Verificatie: `rtk proxy python3 scripts/verify_release.py`. De bron, testfixtures, securityrapporten en testbewijzen staan uitsluitend lokaal. Geen winst-, HFT- of volledige Pine-compatibiliteitsclaim.


De aangeleverde **913sig** is beschikbaar als **913sig OKX · v1** in de Pine Editor. Zie [bron](pine-scripts/913sig-okx.pine) en [gebruik, tellerlogica en weergaveverschil](docs/913sig-okx.md).


### Handmatige OKX-orders

De Terminal bevat nu het native orderticket met Basis, Geavanceerd en Gesplitst (smart_iceberg/TWAP), preview, reserveringen, order-/fill-/feehistorie en afzonderlijke Demo-gateway. De gateway wacht op je zelf ingestelde Demo-key en persoonlijk bevestigde accountlimieten. Live-uitvoering blijft technisch geblokkeerd tot echte Demo-acceptatie en expliciete vrijgave. [Gebruik, ordertypen en grenzen](docs/ORDERS.md). **Start OKX Orders.command** kan de Demo-gateway opnieuw verbinden. De bestaande bots en Pine-worker blijven paper.


### Live-account en percentages

De Terminal toont je echte OKX trading- en Funding-saldi en een 0–100%-slider. Kopen rekent met vrije EUR, verkopen met de gekozen vrije munt. Kies zelf je handelslimieten en geef daarna handmatige Live-orders vrij. Bevestig iedere exacte preview apart. De afzonderlijke `orders-live`-gateway verbindt uitsluitend met de Live-key; paperbots krijgen geen Live-toegang. Zie [orderbediening en huidige grenzen](docs/ORDERS.md).

### Training Lab · neuro-evolutie

Bot Studio bevat nu een lokaal Training Lab met CMA-ES, Laya, datasetversies, eenmalige kwalificatie en interactieve netwerkvisuals. BTC/ETH-historie van twaalf maanden is compleet; het eerste onderzoeksprofiel is gestart. De native route en persoonlijke exacte-versievrijgave zijn gebouwd en geïsoleerd getest. Terminal, trainer en de nieuwe Demo-/Live-gateways zijn geïnstalleerd, de gateways na expliciete toestemming op 6 oktober om 14:39. Echte Demo-, langdurige forward- en resourceacceptatie blijven open; er is geen neurobot vrijgegeven. [Bediening, metingen en resterend werk](docs/TRAINING_LAB.md).

Training Lab kan nu netwerkbestanden exporteren en importeren, ook tussentijds na de eerste berekende generatie. Netwerkarchief bewaart gewichten, normalisatie en parameters; import voegt geen handelsvrijgave toe. Zie [bediening](docs/TRAINING_LAB.md#netwerk-exporteren-en-importeren).
