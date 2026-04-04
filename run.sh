#!/bin/bash
cd "$(dirname "$0")"
source .env
export DEEPGRAM_API_KEY GROQ_API_KEY ORT_DYLIB_PATH

# Activate venv if it exists
if [ -d ".venv" ]; then
  source .venv/bin/activate
fi

# Start Flask web UI in background
python3 web.py &
FLASK_PID=$!

# Cleanup on exit
cleanup() {
  kill $FLASK_PID 2>/dev/null
  pkill -f "audio_engine" 2>/dev/null
  exit 0
}
trap cleanup EXIT INT TERM

EVAL='spawn(fn ->
  wait = fn wait, n ->
    case Process.whereis(Translator.AudioEngine) do
      nil when n > 0 -> Process.sleep(100); wait.(wait, n - 1)
      nil -> IO.puts("AudioEngine not started after 30s")
      _pid -> IO.puts("AudioEngine ready (waiting for Start)")
    end
  end
  wait.(wait, 300)
end)'

if [ "$1" = "--bg" ]; then
  elixir --eval "$EVAL" -S mix run --no-halt
else
  iex --eval "$EVAL" -S mix
fi
