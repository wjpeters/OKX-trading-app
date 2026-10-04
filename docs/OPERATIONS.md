# Lokale operatie en herstel · 0.4.0

## Start, stop en updates

Docker Desktop starten, dan `Start OKX Terminal.command` of `rtk proxy python3 scripts/start.py`. Alleen localhost4780 wordt gepubliceerd. PostgreSQL en Pine hebben geen hostpoort; Pine heeft helemaal geen netwerk. De starthelper activeert uitsluitend aanwezige optionele lokale auth-/Jev-secretmounts. Demo-onboarding is een afzonderlijk expliciet profiel en wordt niet automatisch uitgevoerd.

Browser sluiten stopt workers niet. Mac-slaap/Dockerstop onderbreekt lokale stops. Voor updates: pauzeer instappen en controleer posities/open orders. Er zijn geen blinde automatische updates. Stop via `Stop OKX Terminal.command`; geen `down -v` of globale Docker-prune. Reserves, dagstart/HWM, limieten, bots, kosten en versies zitten duurzaam in PostgreSQL. Herstart verhoogt geen limieten en reset geen balans.

```sh
rtk proxy docker compose ps
rtk proxy docker compose logs --tail=80 terminal
rtk proxy curl --fail http://127.0.0.1:4780/api/ready
```

Health zegt alleen dat het proces leeft. Ready vereist DB/catalogus/publicfeed en levende paperworker. Systeem toont extra data-/klok-/opslagstatus. Bij DB-uitval kan openbare marktdata zichtbaar blijven, maar zijn duurzame uitvoeringsmutaties geblokkeerd. Herstel eerst DB en verse waardering; hervat een risicostop alleen bewust.

## Opslag en logs

Named volumes: PostgreSQL, Pine-jobs en Parquet-archief. Elke 30 s opslagmeting; waarschuwing vanaf90%, instappauze vanaf95% of onbekende meting. Bescherming blijft draaien als daarvoor verse koersen en schrijfbare DB aanwezig zijn. Dit voorkomt geen plotselinge volledige schijf. Containerlogs roteren tot3×10MB per service. Verwijder nooit een databasevolume om ruimte vrij te maken.

## Sleutels en toegang

`.local/` is buiten iCloud/Git/buildcontext, map700/bestanden600. Bootstrap maakt alleen een ontbrekend random DB-password. Vervang dat niet zonder de bijbehorende PostgreSQL-rotatie. Jev/Demo setup vraagt verborgen invoer en schrijft AES256-GCM-envelopes met purpose-AAD. `.local/secrets.key` staat apart van de database en wordt niet in backups opgenomen. Verlies van deze sleutel maakt de opgeslagen providercredentials onleesbaar. Zelfde hostgebruiker kan key plus ciphertext lezen; hostbeveiliging blijft nodig.

`Configureer Jev.command`: nieuwe eigen OpenRouter-key invoeren, daarna in Nieuws budget en inschakeling bevestigen. `Configureer Demo.command`: nieuwe Demo-key/secret/passphrase, uitsluitend read-only accountdetectie en private-kanaalprobe, lokaal rapport `.local/demo-onboarding.json`. Geen order of liveactivering. Verplaats gegevens uit dit rapport niet naar WPOS/cloud.

`Beveilig terminal.command`: nieuw sterk lokaal wachtwoord invoeren; PBKDF2-hash buiten DB. Geldige sessie8h, HttpOnly/SameSiteStrict. Wijziging/rotatie van wachtwoord vereist ook het intrekken van bestaande sessies als die onmiddellijk moeten vervallen. Gebruik deze HTTP-localhostconfiguratie niet als publiek TLS/VPN/MFA-ontwerp.

## Versleutelde backup en restoretest

Een volume is geen backup. Installeer zo nodig de gepinde lokale Python-vereiste via `rtk proxy python3 -m pip install -r scripts/requirements.txt`. Er wordt niets naar cloudopslag geüpload.

```sh
rtk proxy python3 scripts/backup.py backup
rtk proxy python3 scripts/backup.py test
rtk proxy python3 scripts/backup.py verify .local/backups/<bestand>.aesgcm
```

De backup bevat een PostgreSQL custom dump en immutable Parquet plus SHA256-manifest. Sleutel `.local/backup.key` is afzonderlijk,600, **niet** in de backup. Bewaar sleutel en backups op een zelfgekozen beveiligde onafhankelijke locatie; dit is niet automatisch geregeld. Providercredentials en browser-local tekeningen zijn niet inbegrepen. Huidige implementatie heeft een expliciete256 MiB-grens; grotere datasets vereisen streaming/retentie-uitbreiding.

`test` maakt een nieuwe backup en herstelt die naar een willekeurige tijdelijke database plus archiefvolume. Hij controleert SQL-restore, verwachte archiefpaden en alle hashes, en verwijdert uitsluitend deze tijdelijke testresources. De gebruikersdatabase wordt niet overschreven. Bewijs staat in evidence/restore.json. Een echte noodrestore naar een vervangende omgeving vraagt stoppen/pauzeren, expliciete keuze van backup en aparte key, herstellen naar nieuwe DB/volume, controles en pas daarna gecontroleerd omschakelen. Er is geen automatische overschrijfknop.

## Verificatie

```sh
rtk proxy python3 scripts/verify_release.py
rtk proxy python3 scripts/phase3_acceptance.py provision
rtk proxy python3 scripts/phase3_acceptance.py regression
rtk proxy python3 scripts/phase3_acceptance.py studio
rtk proxy python3 scripts/final_checks.py
rtk proxy python3 scripts/auth_acceptance.py
rtk proxy python3 scripts/phase3_acceptance.py cleanup
```

Deze integratieproeven maken een aparte database en terminal op4781, gebruiken virtueel geld en echte publieke data, en raken de rekening op4780 niet aan. Testmanifest `.local/phase3-test.json` begrenst cleanup. Historische fase1/2-scripts zijn geen actuele vrijgave; runtime_checks.py is geblokkeerd omdat die oude proef de hoofd-DB zou stoppen.

`scripts/security_scan.py` exporteert images naar tijdelijke lokale bestanden en scant met Trivy zonder netwerk. De openbare advisorydatabase moet vooraf afzonderlijk opgehaald zijn zonder projectmount. Geen Docker Scout/cloudscan. Linux arm64 is uitgevoerd; amd64 is nog geen geteste platformclaim. Geen remote/VPS-deployment.
