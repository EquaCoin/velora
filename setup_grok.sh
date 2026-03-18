#!/bin/bash
# Script per configurare GROK API

echo "🔧 Configurazione GROK API per Velora"
echo ""

if [ -z "$1" ]; then
    echo "Usage: ./setup_grok.sh <tua_api_key>"
    echo ""
    echo "Ottieni una API key da: https://x.ai/api"
    echo ""
    echo "Esempio:"
    echo "  ./setup_grok.sh xai-xxxxxxxxxxxxxxxx"
    exit 1
fi

API_KEY=$1

# Aggiungi a .bashrc
if ! grep -q "GROK_API_KEY" ~/.bashrc; then
    echo "export GROK_API_KEY=$API_KEY" >> ~/.bashrc
    echo "✅ GROK_API_KEY aggiunto a ~/.bashrc"
else
    sed -i "/GROK_API_KEY/d" ~/.bashrc
    echo "export GROK_API_KEY=$API_KEY" >> ~/.bashrc
    echo "✅ GROK_API_KEY aggiornato in ~/.bashrc"
fi

# Esporta per la sessione corrente
export GROK_API_KEY=$API_KEY

echo ""
echo "🎉 GROK configurato!"
echo "   Provider: GROK (xAI)"
echo "   Modello: grok-2-latest"
echo ""
echo "Per verificare:"
echo "  source ~/.bashrc"
echo "  cd /home/aintel/velora-v3 && cargo run -- tuo_file.vel"
