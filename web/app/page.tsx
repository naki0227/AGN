'use client';

import { useState, useEffect, useRef } from 'react';
import Editor from '@monaco-editor/react';

// Output item type for visual canvas
interface OutputItem {
  type: 'text' | 'card' | 'number';
  content: string;
  color?: string;
  id?: string;
  style?: any;
  styleStr?: string;
}

const CLICKER_GAME_CODE = `// === AGN Clicker Demo ===
メインボタン は "Red" の 箱 だ
メインボタン に "クリックして！" を つなぐ
メインボタン を 画面 に 表示する
スコア は 0 だ

メインボタン を 押したとき
    スコア に 1 を 足す
    "現在のスコア: " を 画面 に 表示する
    スコア を 画面 に 表示する
    
    0.1秒 かけて メインボタン の 色 を blue に する
    0.1秒 かけて メインボタン の サイズ を 0.9倍 に する
    
    0.2秒 後 に
        0.3秒 かけて メインボタン の 色 を Red に する
        0.3秒 かけて メインボタン の サイズ を 1.0倍 に する
    おわり
おわり`;

// Cheat sheet data
const CHEAT_SHEET = [
  { category: '変数定義', examples: ['X は 10 だ', 'メッセージ は "Hello" だ'] },
  { category: '計算', examples: ['X に 5 を 足す', 'X に 3 を 引く', 'X に 2 を 掛ける', 'X に 2 を 割る'] },
  { category: '出力', examples: ['X を 表示する', '"Hello" を 表示する'] },
  { category: '繰り返し', examples: ['10 回 繰り返す\n  X に 1 を 足す\nおわり'] },
  { category: '条件分岐', examples: ['もし X と等しい 5 ならば\n  "正解!" を 表示する\nおわり'] },
  { category: 'UIコンポーネント', examples: ['カード は ぼかした 背景 だ', 'カードに "Hello" を つなぐ'] },
];

// Sample code presets
const SAMPLES: { name: string; code: string }[] = [
  {
    name: '📊 基本計算',
    code: `// 基本計算 - 変数と四則演算
X は 10 だ
X に 5 を 足す
X を 表示する

Y は 20 だ
Y に 2 を 掛ける
Y を 表示する

"計算完了!" を 表示する`
  },
  {
    name: '🔁 繰り返し',
    code: `// 繰り返し処理
カウンター は 0 だ

10 回 繰り返す
  カウンター に 1 を 足す
  カウンター を 表示する
おわり

"ループ終了" を 表示する`
  },
  {
    name: '❓ 条件分岐',
    code: `// 条件分岐
スコア は 85 だ
スコア を 表示する

もし スコア より大きい 80 ならば
  "合格です!" を 表示する
おわり

"判定完了" を 表示する`
  },
  {
    name: '🇬🇧 English Mode',
    code: `// English syntax example
X is 0

repeat 10 times
  add 1 to X
  if X equals 5 then
    show X
  end
end

show X
"Done!" を 表示する`
  },
  {
    name: '🎨 UI コンポーネント',
    code: `// UIコンポーネント定義
カード は ぼかした 背景 だ
カードに "Hello AGN!" を つなぐ

ボタン は Blue な Button だ
ボタン を 表示する

"UIデモ完了" を 表示する`
  },
  {
    name: '🤖 AI 動詞',
    code: `// AI動詞のデモ
テキスト は "Hello World!" だ
テキスト を 表示する

// 翻訳 (デフォルト: 英語)
結果 は テキスト を 翻訳する
結果 を 表示する

// 翻訳 (言語指定: フランス語)
結果2 は テキスト を "フランス語" に 翻訳する
結果2 を 表示する`
  },
  {
    name: '🚀 AGN Playground',
    code: `// --- AGN Playground Sample ---
入力 は "AGNは次世代言語です" だ

// 変数定義と計算
X は 10 だ
X に 5 を 足す
X を 表示する

// UIコンポーネント
カード は ぼかした 背景 だ
カードに 入力 を つなぐ
カード を 表示する`
  },
  {
    name: '🎮 クリッカーゲーム (Demo)',
    code: CLICKER_GAME_CODE
  }

];

export default function Home() {
  const [code, setCode] = useState<string>(SAMPLES[0].code);

  const [logs, setLogs] = useState<string[]>([]);
  const [outputs, setOutputs] = useState<OutputItem[]>([]);
  const [showCheatSheet, setShowCheatSheet] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Draw outputs to canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw each output
    let y = 30;
    outputs.forEach((item, i) => {
      const colors = ['#61dafb', '#f7df1e', '#ff6b6b', '#4ecdc4', '#a855f7'];
      const color = item.color || colors[i % colors.length];

      if (item.type === 'card') {
        // Draw card
        ctx.fillStyle = 'rgba(97, 218, 251, 0.1)';
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.roundRect(20, y, 300, 60, 10);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = '#fff';
        ctx.font = '18px sans-serif';
        ctx.fillText(item.content, 40, y + 38);
        y += 80;
      } else {
        // Draw text/number
        ctx.fillStyle = color;
        ctx.font = item.type === 'number' ? 'bold 32px monospace' : '20px sans-serif';
        ctx.fillText(`${item.type === 'number' ? '📊 ' : '💬 '}${item.content}`, 20, y);
        y += 50;
      }
    });

    // Empty state
    if (outputs.length === 0) {
      ctx.fillStyle = '#444';
      ctx.font = '16px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('Run ▶ をクリックしてコードを実行', canvas.width / 2, canvas.height / 2);
      ctx.textAlign = 'left';
    }
  }, [outputs]);

  // Listen for AGN Animation events
  useEffect(() => {
    const handleAnim = (e: any) => {
      const { target, property, value, duration } = e.detail;
      console.log(`[System] Animation: ${target}.${property} -> ${value} (${duration}s)`);

      // Update outputs state to reflect style change
      setOutputs(prev => prev.map(item => {
        if (item.id === target || (target === 'MainButton' && item.type === 'card')) {
          const newStyle = { ...item.style, transition: `all ${duration}s ease` };

          if (property === '色' || property === 'color') {
            // Simple color map mock
            if (value.includes('青') || value.toLowerCase() === 'blue') newStyle.background = 'rgba(0, 0, 255, 0.4)';
            if (value.includes('赤') || value.toLowerCase() === 'red') newStyle.background = 'rgba(255, 0, 0, 0.4)';
          } else if (property === 'サイズ' || property === 'scale') {
            newStyle.transform = `scale(${value})`;
          } else if (property === '影' || property === 'shadow') {
            if (value > 10) newStyle.boxShadow = '0 20px 40px rgba(0,0,0,0.5)';
            else newStyle.boxShadow = '0 4px 6px rgba(0,0,0,0.1)';
          }

          return { ...item, style: newStyle };
        }
        return item;
      }));
    };

    window.addEventListener('agn-animation', handleAnim);
    return () => window.removeEventListener('agn-animation', handleAnim);
  }, []);

  useEffect(() => {
    // Override console.log/info/error to show in log panel
    const originalLog = console.log;
    const originalInfo = console.info;
    const originalError = console.error;

    const addLog = (msg: string) => setLogs(prev => [...prev.slice(-99), msg]);

    // Capture [Output] messages for visual display
    console.info = (...args) => {
      const msg = args.join(' ');
      addLog(`[INFO] ${msg}`);

      // Parse output messages
      const outputMatch = msg.match(/\[Output\] (.+)/);
      if (outputMatch) {
        let content = outputMatch[1];
        let type = 'text';

        // Check for UI Component pattern [Style Type 'Content' ...]
        let styleStr: string | undefined;
        const componentMatch = content.match(/^\[(.*?) (.*?) '(.*?)'.*\]$/);
        if (componentMatch) {
          type = 'card';
          const [_, s, compType, label] = componentMatch;
          content = label;
          styleStr = s;
        } else if (!isNaN(parseFloat(content)) && isFinite(parseFloat(content))) {
          type = 'number';
        }

        setOutputs(prev => {
          return [...prev, {
            type: type as any,
            content,
            id: type === 'card' ? 'MainButton' : undefined,
            styleStr: styleStr
          }];
        });
      }

      // Parse [Animation] messages
      // [Animation] { "target": "MainButton", "property": "影", "value": "20", "duration": 0.3 }
      const animMatch = msg.match(/\[Animation\] (.+)/);
      if (animMatch) {
        try {
          const animData = JSON.parse(animMatch[1]);
          // Dispatch custom event or update React state?
          // Since we need to update DOM style of overlay, dispatching custom event to window is easiest
          // and having Card component listen?
          // Or update state map of styles.
          const event = new CustomEvent('agn-animation', { detail: animData });
          window.dispatchEvent(event);
        } catch (e) {
          console.error("Failed to parse animation:", e);
        }
      }

      // Parse [RegisterEvent]
      // [RegisterEvent] MainButton click
      const regMatch = msg.match(/\[RegisterEvent\] (\S+) (\S+)/);
      if (regMatch) {
        // We know backend is listening. We just need to make sure UI element triggers handling.
        // Current architecture: `handleRun` sets up wasm. 
        // We need to access `wasm` export from here?
        // `wasm` is loaded dynamically in handleRun. 
        // We should store `wasm` module in ref.
        console.log(`[System] Registered event ${regMatch[2]} on ${regMatch[1]}`);
      }

      originalInfo(...args);
    };

    console.log = (...args) => { addLog(`[LOG] ${args.join(' ')}`); originalLog(...args); };
    console.error = (...args) => { addLog(`[ERR] ${args.join(' ')}`); originalError(...args); };

    return () => {
      console.log = originalLog;
      console.info = originalInfo;
      console.error = originalError;
    };
  }, []);

  const wasmRef = useRef<any>(null);

  const handleRun = async () => {
    // Clear previous outputs
    setOutputs([]);
    setLogs([]);

    try {
      console.info("Starting Wasm execution...");

      try {
        // @ts-ignore
        const wasm = await import(/* webpackIgnore: true */ `/wasm/agn.js?t=${Date.now()}`);
        await wasm.default();
        wasmRef.current = wasm;
        await wasm.run_script(code, "agn-canvas");
      } catch (inner) {
        console.error("Failed to load Wasm:", inner);
      }

    } catch (e: any) {
      console.error("Execution Error:", e);
    }
  };

  return (
    <div style={{ display: 'flex', height: '100vh', flexDirection: 'column', fontFamily: 'sans-serif' }}>
      <header style={{ padding: '0 20px', height: '50px', background: '#20232a', color: '#61dafb', display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid #333' }}>
        <h1 style={{ fontSize: '1.2em', margin: 0 }}>AGN Web IDE 🚀</h1>
        <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <select
            onChange={(e) => setCode(SAMPLES[parseInt(e.target.value)].code)}
            style={{
              padding: '8px 12px',
              background: '#333',
              color: '#fff',
              border: '1px solid #555',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            {SAMPLES.map((sample, i) => (
              <option key={i} value={i}>{sample.name}</option>
            ))}
          </select>
          <button
            onClick={() => setShowCheatSheet(!showCheatSheet)}
            style={{ padding: '8px 15px', background: '#444', color: '#fff', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
          >
            📘 チートシート
          </button>
          <button
            onClick={handleRun}
            style={{
              padding: '8px 20px',
              background: '#61dafb',
              color: '#000',
              fontWeight: 'bold',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '5px'
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = '#4fa8d1'}
            onMouseLeave={(e) => e.currentTarget.style.background = '#61dafb'}
          >
            Run ▶
          </button>
        </div>
      </header>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
          <Editor
            height="100%"
            defaultLanguage="plaintext"
            theme="vs-dark"
            value={code}
            onChange={(value) => setCode(value || '')}
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              lineNumbers: 'on',
              scrollBeyondLastLine: false,
              automaticLayout: true,
            }}
          />
        </div>

        {/* Cheat Sheet Panel */}
        {showCheatSheet && (
          <div style={{
            width: '250px',
            background: '#1e1e1e',
            borderLeft: '1px solid #333',
            overflowY: 'auto',
            padding: '10px'
          }}>
            <h3 style={{ color: '#fff', fontSize: '14px', marginBottom: '10px' }}>📘 AGN チートシート</h3>
            {CHEAT_SHEET.map((section, i) => (
              <div key={i} style={{ marginBottom: '15px' }}>
                <div style={{ color: '#61dafb', fontSize: '12px', fontWeight: 'bold', marginBottom: '5px' }}>{section.category}</div>
                {section.examples.map((ex, j) => (
                  <pre
                    key={j}
                    onClick={() => setCode(prev => prev + '\n' + ex)}
                    style={{
                      background: '#0f0f23',
                      padding: '8px',
                      borderRadius: '4px',
                      fontSize: '11px',
                      color: '#ccc',
                      margin: '5px 0',
                      cursor: 'pointer',
                      whiteSpace: 'pre-wrap',
                      border: '1px solid transparent'
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.borderColor = '#61dafb'}
                    onMouseLeave={(e) => e.currentTarget.style.borderColor = 'transparent'}
                  >
                    {ex}
                  </pre>
                ))}
              </div>
            ))}
            <div style={{ color: '#666', fontSize: '10px', marginTop: '10px' }}>
              クリックでコードに追加
            </div>
          </div>
        )}

        {/* Preview Pane with Hybrid Rendering */}
        <div style={{ width: showCheatSheet ? '50%' : '50%', display: 'flex', flexDirection: 'column', background: '#1e1e1e' }}>
          <div style={{ flex: 1, position: 'relative', background: '#1a1a2e', overflow: 'hidden' }}>
            {/* Canvas for standard drawing */}
            <canvas
              ref={canvasRef}
              id="agn-canvas"
              width={600}
              height={400}
              style={{ width: '100%', height: '100%', objectFit: 'contain', position: 'absolute', top: 0, left: 0 }}
            />

            {/* HTML Overlay for UI Components */}
            <div style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', padding: '20px', pointerEvents: 'none' }}>
              {outputs.map((item, i) => {
                if (item.type === 'card') {
                  return (
                    <div key={i}
                      id={item.id}
                      style={{
                        pointerEvents: 'auto',
                        width: '300px',
                        padding: '20px',
                        background: (item.styleStr === 'Red' || item.styleStr === '赤') ? 'rgba(255,0,0,0.2)' : 'rgba(255, 255, 255, 0.1)',
                        backdropFilter: 'blur(10px)',
                        borderRadius: '16px',
                        border: '1px solid rgba(255, 255, 255, 0.2)',
                        color: 'white',
                        marginTop: '20px',
                        boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
                        transition: 'all 0.3s ease',
                        cursor: 'pointer',
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                        justifyContent: 'center',
                        ...item.style
                      }}
                      onMouseEnter={(e) => {
                        // Only apply default hover if no custom animation overriding
                        if (!item.style?.transform) e.currentTarget.style.transform = 'translateY(-2px)';
                        if (!item.style?.boxShadow) e.currentTarget.style.boxShadow = '0 10px 20px rgba(0, 0, 0, 0.3)';
                      }}
                      onMouseLeave={(e) => {
                        if (!item.style?.transform) e.currentTarget.style.transform = 'translateY(0)';
                        if (!item.style?.boxShadow) e.currentTarget.style.boxShadow = '0 4px 6px rgba(0, 0, 0, 0.1)';
                      }}
                      onClick={() => {
                        if (wasmRef.current) {
                          console.log("Click sent to WASM");
                          wasmRef.current.handle_event(item.id || 'MainButton', 'click');
                        }
                      }}
                    >
                      <div style={{ fontSize: '14px', color: '#aaa', marginBottom: '5px' }}>ぼかした背景</div>
                      <div style={{ fontSize: '18px', fontWeight: 'bold' }}>{item.content || "AGN UI Card"}</div>
                      <div style={{ fontSize: '12px', color: '#888', marginTop: '10px' }}>Interactive Component</div>
                    </div>
                  );
                }
                return null;
              })}
            </div>

            <div style={{ position: 'absolute', top: 10, right: 10, color: '#444', fontSize: '12px' }}>
              Visual Output
            </div>
          </div>

          <div style={{ height: '180px', background: '#111', color: '#ccc', padding: '10px', overflowY: 'auto', fontFamily: 'monospace', fontSize: '11px', borderTop: '1px solid #333' }}>
            <div style={{ color: '#666', borderBottom: '1px solid #333', marginBottom: '5px' }}>Terminal Output</div>
            {logs.map((log, i) => (
              <div key={i} style={{
                color: log.includes('[ERR]') ? '#ff6b6b' : log.includes('[Output]') ? '#4ecdc4' : '#aaa'
              }}>
                {log}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
