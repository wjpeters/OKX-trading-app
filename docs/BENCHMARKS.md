# Werkelijk gemeten · fase 1

Meetmoment UTC: 2026-10-02T07:55:40Z. Host: Apple M1 Pro, macOS-26.6.2-arm64-arm-64bit-Mach-O. Docker Linux arm64, releasebuild, twee toegewezen app-CPU's, max 512 MiB RAM. Twaalf instrumenten, 36 publieke WS-topics, nul bots, nul Jev-calls. Indicatorengine: tien indicatoren over 300 candles per REST-refresh. Andere lokale containers bleven draaien; geen geïsoleerde benchmarkhost.

| Meting | p50 | p95 | p99 | Samples |
|---|---:|---:|---:|---:|
| WS-schema-validatie + in-memory update | 0.020292 ms | 0.084958 ms | 0.273916 ms | 2048 |
| Lokale HTTP /api/status roundtrip, inclusief DB-probe | 1.575333 ms | 2.855959 ms | 4.475625 ms | 50 |

Dit zijn beperkte looptijdmetingen, **geen strategie/event→intent-benchmark, geen orderlatency en geen HFT-claim**. De <10ms p95-doelstelling uit de specificatie kan pas met de volledige native strategie- en riskroute beoordeeld worden. Data-ontvangst/netwerk, durable intent-write, Jev, order-submit/ack/fill zitten niet in de WS-sample. Events per seconde en foutpercentage onder gecontroleerde burst/soak zijn nog niet gemeten. De UI toont dezelfde scope.

## Candleproef

- 1m: 300 echte candles, 0 gaten, indicatorberekening 0.0490 ms; gemeten REST-pad 83.24 ms.
- 5m: 300 echte candles, 0 gaten, indicatorberekening 0.0562 ms; gemeten REST-pad 68.71 ms.
- 15m: 300 echte candles, 0 gaten, indicatorberekening 0.0510 ms; gemeten REST-pad 127.35 ms.
- 1H: 300 echte candles, 0 gaten, indicatorberekening 0.2668 ms; gemeten REST-pad 135.39 ms.
- 4H: 300 echte candles, 0 gaten, indicatorberekening 0.0493 ms; gemeten REST-pad 276.52 ms.
- 1Dutc: 300 echte candles, 0 gaten, indicatorberekening 0.0543 ms; gemeten REST-pad 81.59 ms.

De coverage in evidence/smoke.json bevat exacte eerste/laatste UTC-timestamps. Een afzonderlijke history-candles-pagina met after-cursor bevat uitsluitend oudere bars. Dit is geen bewijs van onbeperkte historie. REST-padmeting omvat limiterwachttijd/parse/rekenwerk en is daarom geen zuivere netwerklatency.

Geen resultaten, P&L, fills of confidencewaarden zijn voor de interface verzonnen. Testfixtures voor indicatorrekenregels zijn uitsluitend unit-testdata.

## Fase 2 · native strategie/risk en duurzame paperroute

Gemeten op dezelfde Apple M1 Pro, Docker Linux arm64, gepinde Rust-releasebuild. Er draaiden ook andere lokale containers. 5.000 seriële fixture-evaluaties, één instrument, één wallet, SMA 20/50, Decimal-sizing en complete entry-riskpreview. Geen PostgreSQL, WS-decode, netwerk, AI of echte order binnen deze native sample. De fixture is geen marktresultaat.

| Scope | p50 | p95 | p99 | Samples |
|---|---:|---:|---:|---:|
| SMA + sizing + riskpreview | 0.002292 ms | 0.002500 ms | 0.003750 ms | 5000 |

Een werkelijk bevestigde lokale paperorder in de geïsoleerde integratietest gebruikte echte publieke boekdata: HTTP-roundtrip 4.179 ms; gemeten serverpad inclusief risicocontrole, DB-reservering en commit 2.953 ms. Dit is **één sample, geen percentielclaim**. Ack/fill volgen later: minimaal 500 ms modelvertraging plus workerintervallen/IO. Burst/soak en volledige end-to-end-percentielen zijn niet uitgevoerd. Het volledige event→intent-doel uit de prompt is hiermee niet als productie-SLO gevalideerd.

Exact bewijs: [phase2-benchmark.json](../evidence/phase2-benchmark.json), [unituitvoer](../evidence/phase2-unit-tests.txt) en [integratie-uitvoer](../evidence/phase2-integration.json).

## Fase4 · minimale runtime en echte korte belastingproef

Bewijs: [phase4-benchmark.json](../evidence/phase4-benchmark.json). Apple M1 Pro, macOS26.6.2, DockerLinux arm64, app2CPU/512MiB,12 instrumenten,0 actieve gebruikersbots. Tien native indicatoren per opgevraagde reeks; vier UI-charts waren tijdens de eindcontrole beschikbaar. Andere lokale containers bleven draaien. Geen betaalde Jev-call.

30.40s observatie, 4651 gevalideerde WS-berichten (152.98/s). WS-validatie+in-memoryupdate p50/p95/p99 = 0.229/0.754/2.183ms,2048 samples. Dit omvat bookdeltareconstructie maar geen duurzame orderroute.

30 lokale statusrequests: p50/p95/p99=5.90/28.26/33.35ms,0 fouten. Begrensde burst in de disposable testomgeving:120 requests,12 gelijktijdig,p95=16.70ms,0 fouten. Geen langlopende soak of bewijs van lekvrij geheugen.

Docker rapporteerde appgeheugen8,40→8,55MiB, Pine12,09→12,31MiB en DB66,53→66,48MiB in deze korte proef; dit is geen maximale workloadmeting. Pinepreview/backtestduur staat per werkelijk uitgevoerd rapport; Jev-latency ontbreekt zonder echte call. De <10ms volledigeevent→intent-doelstelling is hiermee niet volledig gevalideerd.
