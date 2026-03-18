# Velora v3 - AI-First Edition 🚀

## Cos'è Velora AI-First?

Velora è un linguaggio di programmazione che si **auto-evolve** tramite AI. Scrivi cosa vuoi in linguaggio naturale, e Velora genera il codice per te!

```vel
main:
    print("Demo Velora AI")
    # AI: crea funzione per calcolare il fattoriale
    print(fattoriale(5))  // Output: 120
```

## 🎯 Caratteristiche

### 1. Auto-Evoluzione con Template
Quando scrivi `# AI: descrizione`, Velora:
1. Cerca un template predefinito che matcha
2. Se trovato, inserisce il codice pronto all'uso
3. Se non trovato, usa un fallback generico

### 2. Retry con Validazione (Opzionale)
Se configuri un API key (OpenAI/Claude), Velora:
1. Prova a generare codice con l'AI
2. Valida sintassi automaticamente
3. Se fallisce, riprova con feedback (max 3 tentativi)
4. Se AI non disponibile, usa template

### 3. Template Inclusi

| Template | Descrizione | Pattern riconosciuti |
|----------|-------------|---------------------|
| `sum_to_n` | Somma 1 a N | "somma", "sum", "1 a n" |
| `factorial` | Fattoriale | "fattoriale", "factorial", "n!" |
| `is_prime` | Verifica primo | "primo", "prime" |
| `fibonacci` | Sequenza Fibonacci | "fibonacci", "fib" |
| `power` | Potenza | "potenza", "power", "^" |
| `double` | Raddoppia | "doppio", "double" |
| `gcd` | MCD | "mcd", "gcd" |
| `countdown` | Conto rovescia | "countdown", "timer" |
| `hello` | Saluto | "ciao", "hello", "saluta" |

## 🚀 Utilizzo

### Installazione
```bash
git clone <repo>
cd velora-v3
cargo build --release
```

### Esempio Base
```bash
# Crea file test.vel
cat > test.vel << 'EOF'
main:
    print("Inizio")
    # AI: crea funzione per somma 1 a 10
    # AI: verifica se 17 è primo
    print("Fine")
EOF

# Esegui
cargo run -- test.vel
```

### Output
```
Inizio
55      ← somma 1-10
1       ← 17 è primo
Fine
```

## ⚙️ Configurazione AI (Opzionale)

Per usare AI esterna invece dei template:

```bash
# OpenAI
export OPENAI_API_KEY="sk-tua-chiave"

# Anthropic Claude
export ANTHROPIC_API_KEY="sk-ant-tua-chiave"

# Ollama (locale)
export AI_PROVIDER=ollama
export OLLAMA_MODEL=nemotron-mini-4b
```

## 🛠️ Architettura

```
Codice Velora
     ↓
Parser (# AI: ...)
     ↓
AI Module
 ├─ Prova AI (se configurata)
 ├─ Validazione codice
 ├─ Retry con feedback
 └─ Fallback Template
     ↓
Codice generato
     ↓
Interprete Velora
     ↓
Output
```

## 📝 Sintassi Velora

```vel
# Funzioni
fn nome(x) -> Int {
    return x * 2
}

# Variabili
let x = 10

# Controllo
if x > 5 {
    print("grande")
} else {
    print("piccolo")
}

# Loop
while x > 0 {
    print(x)
    let x = x - 1
}

# Main
main:
    print("Hello World")
```

## 🎓 Esempi

### Esempio 1: Matematica
```vel
main:
    # AI: crea funzione potenza
    print(power(2, 10))  # 1024
```

### Esempio 2: Stringhe
```vel
main:
    # AI: saluta l'utente
    print(greet("Mondo"))  # Ciao Mondo!
```

### Esempio 3: Combinato
```vel
# AI: crea funzione per media di tre numeri

main:
    let m = media(10, 20, 30)
    print(m)  # 20
```

## 📊 Performance

- **Parsing**: < 10ms
- **Generazione template**: < 1ms
- **Generazione AI**: 1-5s (dipende dal provider)
- **Esecuzione**: ~1000 ops/sec

## 🔮 Roadmap

- [x] Sistema template robusto
- [x] Validazione automatica
- [x] Retry con feedback
- [ ] Fine-tuning modelli locali
- [ ] Template dinamici/user-defined
- [ ] IDE integration (VSCode)
- [ ] Package manager

## 🤝 Contribuire

Aggiungi nuovi template in `src/ai/ai_templates.rs`:

```rust
CodeTemplate {
    name: "nome_funzione",
    description: "Cosa fa",
    pattern: "keyword1|keyword2|keyword3",
    code: r#"fn nome() -> Int {
    return 42
}
print(nome())"#,
}
```

## 📄 Licenza

MIT License - Vedi LICENSE

---

**Velora AI-First**: Programmare non è più scrivere codice, ma descrivere cosa vuoi! 🚀
