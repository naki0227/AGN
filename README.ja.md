# AGN (Antigravity-Native) 🚀

**The world's first AI-native, multilingual programming language powered by Rust and GPU.**
**「AIが知能となり、GPUが翼となる。重力を無視する次世代の開発体験を。」**

---

## 🌟 What is AGN?

AGN (Antigravity-Native) は、自然言語（日本語・英語）の直感性と、低レイヤ（Rust/LLVM）の圧倒的パフォーマンスを融合させた次世代のプログラミング言語です。

Google Antigravity 環境での開発を前提に設計されており、AIエージェントが「意図」を理解してコードを最適化します。

### 💎 Key Features

* **Multilingual Native Syntax**: 日本語（SOV）と英語（SVO）を等価に扱い、同一の論理構造（Unified AST）に変換します。
    * *JP:* `X を 並列で 表示する`
    * *EN:* `parallel show X`
* **AI as a First-class Citizen**: AI推論が標準の「動詞」として組み込まれています。
    * `結果 は 入力 を 要約する` (Summarize input into result)
* **GPU-Accelerated Universal UI**: `wgpu` をバックエンドに採用し、Dribbble/Rive級のリッチなUIを60fpsで描画します。
* **Antigravity-Speed**: LLVM IR出力を通じて、Rust/C++に匹敵するネイティブバイナリを生成します。

---

## 🚀 Quick Start

### 📋 Prerequisites
- Rust (latest stable)
- LLVM 15+
- Google Antigravity IDE (Recommended)

### 🛠 Installation & Run
```bash
git clone https://github.com/naki0227/AGN.git
cd AGN
cargo run -- examples/demo_phase12.agn --run-compiled
```

## 🗺 Roadmap
- [x] Phase 1-3: Core Kernel & LLVM Backend
- [x] Phase 4-6: Multilingual SVO & Universal UI (Wasm/Native)
- [x] Phase 7-12: GPU Rendering (wgpu) & Interactive Animations
- [ ] Phase 13: Mobile Native Support (iOS/Android)
- [ ] Phase 14: AI-driven Auto-Refactoring

## 🤝 Contribution
AGNは「世界一」を目指すオープンソースプロジェクトです。 バグレポート、機能提案、プルリクエストを歓迎します！

Developer: naki0227
Portfolio: enludus.vercel.app

## 📄 License
This project is licensed under the MIT License - see the LICENSE file for details.
