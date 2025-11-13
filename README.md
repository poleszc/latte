# Latte - Piotr Oleszczuk, nr 448390
## Budowanie projektu
1) Instalacja Rust:
   - Aby zainstalować należy uruchomić skrypt `build.sh` znajdujący się w katalogu głównym projektu.
    Skrypt instaluje Rust i Cargo (package manager) z oficjalnej strony Rusta i ładuje zmiany
    żeby były dostępne z aktualnej sesji terminalowej, a następnie wykonuje polecenie `make`, które
    pobierze wszystkie potrzebne dependencje, zbuduje projekt i przekopiuje pliki wykonywalne
    do katalogu głównego.
2) Budowanie projektu:
   - Projekt sam skompiluje się po wykonaniu skryptu `build`. Jeżeli jednak zajdzie potrzeba ponownej
    kompilacji (pomijajac już samą instalacje Rust) w katalogu głównym projektu trzeba uruchomić `make`,
    który skompiluje projekt i przekopiuje pliki wykonywalne `insc_jvm` oraz `insc_llvm` do katalogu głównego.
    - **Dodatkowe opcje:**
      - `make build-jvm` - przebuduje jedynie binarkę z backendem Jasminowym
      - `make build-llvm` - przebuduje jedynie binarkę z backendem LLVM
      - `make clean` - usuwa zbudowane binarki oraz pliki `*.{j, class, ll, bc}` z katalogu `tests/in`
      - `make clean-all` - usuwa to co `make clean` oraz wszystkie pliki depencencji

## Użycie kompilatora
Domyślna kompilacja:
```bash
# Skompiluje plik test01.ins. Pliki `*.j` oraz '*.class` domyślnie znajdą się tej samej lokalizacji co plik kompilowany.
./insc_jvm tests/in/test01.ins 
# Analogicznie dla backendu LLVM. 
./insc_llvm tests/in/test01.ins
```

Uruchomienie programu po kompilacji:
```bash
# Skompiluje plik test01.ins oraz uruchomi JVM na skompilowanym pliku.
./insc_jvm tests/in/test01.ins --run
# Skompiluje plik test01.ins oraz uruchomi lli na skompilowanym pliku.
./insc_llvm tests/in/test01.ins --run
```

Specyfikowanie ścieżki wyjściowej:
```bash
# Skompiluje plik test01.ins. Pliki pośrednie znajdą się w katalogu `output`. Bez tej opcji 
# pliki pośrednie znajdą się tam gdzie plik kompilowany.
./insc_jvm tests/in/test01.ins --output-dir output
# Analogicznie dla backendu LLVM. 
./insc_llvm tests/in/test01.ins
```

## Struktura projektu
```bash
.
├── build.rs            # Skrypt wykonujący się automatycznie przez rozpoczęciem kompilacji. Buduje parser `lalrpop` na podstawie gramatyki lalrpop oraz automatycznie generuje testy.
├── build.sh            # Skrypt instalujący Rust i budujący projekt.
├── Cargo.lock
├── Cargo.toml          # Konfiguracja projektu, dependencje itp.
├── Instant.cf          # Gramatyka BNCF. W tym projekcie nie używana.
├── instant.md          # Treść zadania
├── lib
│   ├── jvm
│   │   └── jasmin.jar
│   └── llvm
│       ├── runtime.bc
│       └── runtime.ll
├── Makefile
├── README.md
├── src
│   ├── bin
│   │   ├── insc_jvm.rs     # Binarka uruchamiająca kompilacje z backendem Jasmine.
│   │   └── insc_llvm.rs    # Binarka uruchamiająca kompilacje z backendem LLVM.
│   ├── config.rs           # Definicja konfiguracji kompilatora (argumentów linii poleceń). Tutaj użyta jest biblioteka CLAP.
│   ├── diagnostics.rs      # Produkcja ładnych błędów kompilacji za pomocą biblioteki Ariadne.
│   ├── driver.rs           # Główny silnik kompilacji, czyta pliki, wywołuje parsowanie, tworzy katalogi na pliki wyjściowe. 
                            # Kompiluje sparsowany program podanym backendem.
│   ├── error.rs            # Definicje błędów kompilacji (błędy syntaktyczne, semantyczne, błędy wykonania polecen zewnętrznych, itp.)
│   ├── instant.lalrpop     # Gramatyka języka instant w formacie lalrpop.
│   ├── jvm
│   │   ├── driver.rs       # Główny silnik zarządzający kompilacją backendem Jasmine. Wykonuje kompilacje, 
                            # uruchamia jasmine, uruchamia program na JVM (jeżeli taka opcja została przekazana).
│   │   ├── instructions.rs # Abstrakcja nad instrukcjami JVM. 
│   │   ├── lowering.rs     # Główna logika kompilacji. Struktura JasmineGenerator zarządza całym przekształceniem 
                            # AST w kod Jasmine.
│   │   └── mod.rs          # Definicje funkcji/struktur eksportowanych z modułu
│   ├── lib.rs
│   ├── llvm # Zawartość tego projektu jest analogiczna jak w przypadku `src/jvm` tylko dla LLVM.
│   │   ├── driver.rs
│   │   ├── instructions.rs
│   │   ├── lowering.rs
│   │   └── mod.rs
│   └── parser
│       ├── ast.rs      # Definicja drzewa AST i funkcji liczących głębokości poddrzew.
│       ├── lexer.rs    # Implementacja leksera.
│       ├── mod.rs
│       └── tokens.rs # Definicja struktury tokenów.
└── tests
    ├── common.rs # Definicja funkcji do testów automatycznych.
    ├── compiler_tests.rs
    ├── in
    │   ├── bad_01.ins
    │   ├── bad_02.ins
    │   ├── bad_03.ins
    │   ├── bad_04.ins
    │   ├── bad_05.ins
    │   ├── bad_06.ins
    │   ├── error_01.ins
    │   ├── error_02.ins
    │   ├── error_03.ins
    │   ├── error_04.ins
    │   ├── test01.ins
    │   ├── test02.ins
    │   ├── test03.ins
    │   ├── test04.ins
    │   ├── test05.ins
    │   ├── test06.ins
    │   ├── test07.ins
    │   └── test08.ins
    └── out
        ├── test01.output
        ├── test02.output
        ├── test03.output
        ├── test04.output
        ├── test05.output
        ├── test06.output
        └── test07.output

```

## Użyte biblioteki
- [Clap](https://crates.io/crates/clap) - parser argumentów podanych z wiersza lini poleceń.
- [Lalrpop](https://lalrpop.github.io/lalrpop/index.html) - generator parserów dla języka Rust. Gramatyka **lalrpop** użyta w tym projekcie znajduje się [tutaj](./src/instant.lalrpop).
- [Logos](https://crates.io/crates/logos) - generator lekserów dla języka Rust. 
- [Ariadne](https://crates.io/crates/ariadne) - biblioteka do generacji ładnych błędów kompilacji.
- [tempfile](https://crates.io/crates/tempfile) - biblioteka do zarządzania plikami tymczasowymi w systemie. W tym projekcie używana do testów integracyjnych.