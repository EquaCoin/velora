# Velora AI-First con GROK Integration 🤖

## Architettura del Sistema AI

Velora utilizza un **sistema a cascata** per l'auto-evoluzione:

```
Richiesta AI
    ↓
[1] Template Statici (istantaneo)
    ↓ (se non trovato)
[2] Cache Locale (istantaneo)
    ↓ (se non trovato)
[3] GROK API (1-3 secondi)
    ↓ (se fallisce)
[4] Fallback Generico (sempre disponibile)
```

## Provider Supportati

| Provider | Velocità | Requisiti | Use Case |
|----------|----------|-----------|----------|
| **Template Statici** | ~0ms | Nessuno | Operazioni comuni (somma, fattoriale, etc.) |
| **Cache Locale** | ~0ms | File `.velora_template_cache` | Template generati precedentemente |
| **GROK (xAI)** | ~1-3s | `GROK_API_KEY` | Codice custom non nei template |
| **Fallback** | ~0ms | Nessuno | Fallback garantito |

## Configurazione GROK

### 1. Ottieni API Key
- Vai su https://x.ai/api
- Crea un account
- Genera una API key

### 2. Configura Velora
```bash
# Opzione A: Script automatico
./setup_grok.sh xai-la-tua-chiave

# Opzione B: Manuale
export GROK_API_KEY="xai-la-tua-chiave"
echo 'export GROK_API_KEY="xai-la-tua-chiave"' >> ~/.bashrc
```

### 3. Verifica
```bash
cargo run -- test.vel
# Dovresti vedere: "🧠 Genero con GROK..."
```

## Template Statici Inclusi

Operazioni matematiche:
- `sum_to_n` - Somma da 1 a N
- `factorial` - Fattoriale
- `is_prime` - Verifica numero primo
- `fibonacci` - Sequenza Fibonacci
- `power` - Potenza (base^exp)
- `gcd` - Massimo Comun Divisore
- `double` - Raddoppia valore

Utility:
- `countdown` - Conto alla rovescia
- `hello_function` - Funzione saluto

## Sistema di Cache

I template generati da GROK vengono salvati automaticamente in `.velora_template_cache`.

### Formato Cache
```
nome_template|pattern|use_count|codice
```

### Esempio
```
cube|cube|potenza|1|fn cube(x) -> Int {\n    return x * x * x\n}\nprint(cube(3))
```

### Benefici
- **Velocità**: I template generati sono istantanei al riuso
- **Affidabilità**: Template validati sono garantiti funzionanti
- **Apprendimento**: Il sistema migliora con l'uso

## Flusso di Generazione

### Esempio 1: Template Esistente
```vel
# AI: somma da 1 a 10
```
Output:
```
🤖 Auto-evoluzione: somma da 1 a 10
   📋 Template trovato: sum_to_n
✅ Codice generato!
```

### Esempio 2: Cache Hit
```vel
# AI: calcola fattoriale
```
Output:
```
🤖 Auto-evoluzione: calcola fattoriale
   💾 Template dalla cache: factorial
✅ Codice generato!
```

### Esempio 3: GROK Generation
```vel
# AI: crea funzione per media di tre numeri
```
Output:
```
🤖 Auto-evoluzione: crea funzione per media di tre numeri
   🧠 Genero con GROK...
   ✅ Nuovo template salvato in cache!
✅ Codice generato!
```

### Esempio 4: Fallback
```vel
# AI: crea funzione per ordinare array
```
Output:
```
🤖 Auto-evoluzione: crea funzione per ordinare array
   🧠 Genero con GROK...
   ⚠️  GROK non disponibile
   📋 Uso fallback generico
⚠️  Codice generico (modificare manualmente)
```

## Aggiungere Template Statici

Modifica `src/ai/ai_templates.rs`:

```rust
CodeTemplate {
    name: "nome_funzione",
    description: "Cosa fa",
    pattern: "keyword1|keyword2|keyword3",
    code: r#"fn nome(x) -> Int {
    return x * 2
}
print(nome(5))"#,
},
```

## Debug

### Verifica Provider Configurati
```bash
# Aggiungi a main.rs temporaneamente
println!("Providers: {:?}", velora::ai::list_providers());
```

### Test GROK Diretto
```bash
curl https://api.x.ai/v1/chat/completions \
  -H "Authorization: Bearer $GROK_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "grok-2-latest",
    "messages": [{"role": "user", "content": "Say hello"}]
  }'
```

## Prestazioni

| Operazione | Tempo Tipico |
|------------|--------------|
| Template statico | < 1ms |
| Cache hit | < 1ms |
| GROK (prima volta) | 1-3s |
| GROK (cache) | < 1ms |
| Fallback | < 1ms |

## Limitazioni

1. **GROK richiede internet**
2. **Rate limiting**: xAI ha limiti sulle chiamate API
3. **Validazione**: Il codice GROK viene validato, ma potrebbe non essere ottimale
4. **Sintassi**: GROK potrebbe generare sintassi non supportata (viene scartata)

## Troubleshooting

### "GROK non configurato"
```bash
export GROK_API_KEY="xai-tua-chiave"
```

### "Codice GROK non valido"
- GROK ha generato sintassi non supportata
- Riprova con una descrizione diversa
- Oppure il sistema usa fallback

### "Rate limit exceeded"
- Attendi qualche secondo
- Il sistema passerà automaticamente a fallback

## Roadmap

- [x] Template statici
- [x] Cache locale
- [x] GROK integration
- [ ] OpenAI/Claude come alternativa a GROK
- [ ] Fine-tuning modelli locali
- [ ] Template community (condivisione)

---

**Velora AI-First**: Il linguaggio che impara con te! 🚀
