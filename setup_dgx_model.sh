#!/bin/bash
# Setup ottimizzato per modelli LLM su NVIDIA DGX Spark
# Configura un modello coding ottimizzato per auto-evoluzione Velora

set -e

echo "🚀 NVIDIA DGX Spark - Model Setup"
echo "=================================="
echo ""

# Verifica Ollama
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama non trovato. Installazione..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

echo "✅ Ollama trovato: $(ollama --version)"
echo ""

# Verifica DGX Spark
if command -v nvidia-smi &> /dev/null; then
    echo "🎮 GPU rilevata:"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader | head -1
    echo ""
fi

# Modelli disponibili per coding su DGX Spark
declare -A MODELS=(
    ["1"]="codellama:7b-code-q4_K_M - Veloce, ottimo per codice"
    ["2"]="qwen2.5-coder:7b-q4_K_M - Eccellente per coding multi-lingua"
    ["3"]="deepseek-coder:6.7b-q4_K_M - Specializzato in codice"
    ["4"]="phi4:14b-q4_K_M - Microsoft, buon bilanciamento"
    ["5"]="nemotron-mini-4b - Nativo NVIDIA, ultra-veloce"
)

echo "📦 Modelli disponibili per DGX Spark:"
echo ""
for key in "${!MODELS[@]}"; do
    echo "   $key. ${MODELS[$key]}"
done
echo ""

# Selezione modello
MODEL_CHOICE="${1:-1}"
if [ -z "$MODEL_CHOICE" ] || [ -z "${MODELS[$MODEL_CHOICE]}" ]; then
    read -p "Seleziona modello (1-5) [default: 1]: " MODEL_CHOICE
    MODEL_CHOICE="${MODEL_CHOICE:-1}"
fi

MODEL_INFO="${MODELS[$MODEL_CHOICE]}"
MODEL_NAME=$(echo "$MODEL_INFO" | cut -d' ' -f1)

echo ""
echo "📥 Download modello: $MODEL_NAME"
echo "   Questo potrebbe richiedere alcuni minuti..."
echo ""

ollama pull "$MODEL_NAME"

echo ""
echo "✅ Modello scaricato!"
echo ""

# Crea Modelfile ottimizzato per DGX Spark
cat > Modelfile.velora << 'EOF'
FROM {{MODEL}}

# Parametri ottimizzati per coding su DGX Spark
PARAMETER temperature 0.15
PARAMETER top_p 0.85
PARAMETER top_k 40
PARAMETER num_ctx 4096
PARAMETER num_predict 500
PARAMETER repeat_penalty 1.1

# Ottimizzazioni GPU DGX Spark
PARAMETER num_gpu 999
PARAMETER num_thread 8

SYSTEM """You are a Velora code generator. Output ONLY valid Velora code.

CRITICAL RULES:
1. Output ONLY code, NO explanations, NO markdown, NO comments
2. Use EXACT syntax:
   - Variables: let x = 10 (NO mut, NO type annotations)
   - Print: print(x) (NO println)
   - Functions: fn name(x) -> Int { return x * 2 }
   - Types: Int, String, Bool
   - If: if x > 5 { print("yes") } else { print("no") }
   - While: while x > 0 { print(x); let x = x - 1 }
3. NO := operator
4. NO // or /* */ comments
5. Use + for string concatenation
6. Function calls: name(arg1, arg2)

EXAMPLE VALID CODE:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
print(factorial(5))

Generate ONLY Velora code for the request."""
EOF

# Sostituisci il modello nel Modelfile
sed -i "s|{{MODEL}}|$MODEL_NAME|g" Modelfile.velora

# Crea modello personalizzato
custom_model_name="velora-coder"
echo "🔧 Creazione modello personalizzato: $custom_model_name"
ollama create "$custom_model_name" -f Modelfile.velora

echo ""
echo "✅ Modello '$custom_model_name' creato!"
echo ""

# Test modello
echo "🧪 Test del modello..."
echo "   Richiesta: 'crea funzione per raddoppiare un numero'"
echo ""

test_output=$(ollama run "$custom_model_name" "crea funzione per raddoppiare un numero" 2>&1)
echo "   Output: $test_output"
echo ""

# Verifica se l'output contiene codice valido
if echo "$test_output" | grep -q "fn.*->.*Int"; then
    echo "✅ Test PASSATO - Il modello genera codice valido!"
else
    echo "⚠️  Il modello potrebbe generare testo spiegativo."
    echo "   Prova con un altro modello o affina il prompt."
fi

echo ""
echo "=================================="
echo "🎉 Setup completato!"
echo ""
echo "Per usare il modello con Velora:"
echo "   OLLAMA_MODEL=$custom_model_name cargo run -- example_ai.vel"
echo ""
echo "Oppure imposta come default:"
echo "   echo 'export OLLAMA_MODEL=$custom_model_name' >> ~/.bashrc"
echo ""

# Pulizia
rm -f Modelfile.velora
