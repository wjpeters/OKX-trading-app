# OKX Tradingterminal

Nieuw lokaal project volgens [BUILD_SPEC.md](BUILD_SPEC.md), gestart op 2 oktober 2026. Broncode, gegevens en sleutels blijven in `/Users/wp/WPOS-local/projects/okx-tradingterminal`, buiten iCloud. Geen hergebruik van eerdere tradingapps en geen clouddeployment.

## Starten

1. Start Docker Desktop.
2. Dubbelklik **Start OKX Terminal.command**.
3. Open http://127.0.0.1:4780.

De app start als **PAPER_LOCAL**. Je bevestigt zelf een virtuele EUR-rekening en de rekening-/botlimieten in Paper. Er is geen gebruikersrekening of gebruikersbot door de ontwikkeltests geactiveerd. Jev mag voorstellen geven; alleen de vaste risicokern kan deze na hercontrole en duurzame reservering omzetten in paperorders.

```sh
rtk proxy python3 scripts/start.py
```

Stoppen via **Stop OKX Terminal.command**. Gebruik geen `down -v`: dat verwijdert de database. Browser sluiten laat bots en alerts doorlopen; Mac-slaap of Dockerstop onderbreekt lokale bescherming.

## Beschikbare werkruimtes

- Terminal: echte OKX EEA-prijzen, orderboek met sequencecontrole, tape, één tot vier gekoppelde charts, tien configureerbare indicatoren, tekeningen en opgeslagen workspaces.
- Paper: vaste risico-/verlieslimieten, preview/bevestiging, orderbeheer, partial fills, posities, stops en serverbots.
- Pine Editor: Monaco, echte PineTS-parser, inputs, diagnostiek, versies, plots en candle-backtests. Het expliciet begrensde closed-bar-profiel staat in [PINE_COMPATIBILITY.md](PINE_COMPATIBILITY.md).
- Bot Studio: bewerkbare nodegrafiek en canonieke configuratie, templates en versiebeheer. Een gepinde strategie kan aan een gepauzeerde paperbot zonder open exposure worden gekoppeld. Starten is een afzonderlijke actie.
- Alerts: prijs, indicator, vastgelegde tekenlijn, bot en risicostatus; servercontrole en duurzame lokale inbox.
- Evaluatie: Parquet-import, hervatten, walk-forward, parametergevoeligheid en vergelijking van werkelijk geregistreerde paperbots met/zonder Jev.
- Nieuws: OKX announcements/status, dagelijkse Bitcoin Fear & Greed, bronstatus, Jev-kosten en ontvangen voorstellen. Ontbrekende gegevens blijven zichtbaar als ontbrekend.
- Systeem: verbindingen, latency, opslagbewaking, bescherming en vrijgavevoorwaarden.

## Tekenen op de grafiek

Kies links een pictogram of open **Tekentools** voor uitleg. Klik de aangegeven punten aan en sleep daarna de vorm of de ronde punten om te bewerken. Esc annuleert; ⌘/Ctrl Z maakt ongedaan. Zie de [tekenhandleiding](docs/DRAWING_TOOLS.md).

## Externe aansluitingen

**Configureer Jev.command** vraagt een nieuwe OpenRouter-key via verborgen lokale invoer. Daarna bevestig je het AI-kostenbudget in Nieuws. Er zijn nog geen betaalde calls uitgevoerd. Nooit keys in chat plakken of uit eerdere projecten overnemen.

**Configureer Demo.command** slaat afzonderlijke Demo-credentials versleuteld op en voert uitsluitend accountdetectie en een private-kanaalprobe uit. Dit activeert geen orders. **De volledige Demo/LIVE-uitvoering is nog niet af**: continue private reconciliatie, de duurzame exchange-orderadapter, native bescherming en fencing moeten nog worden gebouwd/geaccepteerd. Zie [STATUS.md](STATUS.md) en [resterend werk](docs/REMAINING.md). LIVE kan niet worden aangezet in deze release.

**Beveilig terminal.command** stelt optionele lokale wachtwoordbeveiliging in. Backups maak/test je met `rtk proxy python3 scripts/backup.py test`.

## Handleidingen en bewijs

**Vervolgplan:** [Fase 5 en volledige afronding](docs/PHASE5_EN_AFRONDING.md), opgesteld op 4 oktober 2026. Bouwvolgorde, afhankelijkheden, acceptatiecriteria en benodigde persoonlijke invoer. Dit plan activeert geen handel.

[Gebruik van Pine/Bot Studio/Jev](docs/PHASE3_4.md) · [Risico en paper](docs/PHASE2.md) · [Architectuur](docs/ARCHITECTURE.md) · [Capabilities](docs/CAPABILITIES.md) · [API](docs/API.md) · [Operatie/herstel](docs/OPERATIONS.md) · [Threat model](docs/THREAT_MODEL.md) · [Acceptatie](docs/ACCEPTANCE.md) · [Providers/licenties](docs/PROVIDERS_LICENSES.md) · [Metingen](docs/BENCHMARKS.md).

Exacte dependencies staan in lockfiles; basisimages op digest. Verificatie: `rtk proxy python3 scripts/verify_release.py`. De bron, testfixtures, securityrapporten en testbewijzen staan uitsluitend lokaal. Geen winst-, HFT- of volledige Pine-compatibiliteitsclaim.
