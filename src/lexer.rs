//! AGN Lexer - 字句解析器
//! 日本語・英語の両方をトークンとして認識する

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// 名詞（変数名）
    Noun(String),
    /// 数値リテラル
    Number(f64),
    /// 文字列リテラル
    String(String),
    /// 動詞（関数名）
    Verb(String),
    /// 助詞「は」- 主題マーカー
    ParticleWa,
    /// 助詞「に」- 間接目的語マーカー
    ParticleNi,
    /// 助詞「を」- 直接目的語マーカー
    ParticleWo,
    /// 助詞「だ」- 断定（代入）
    ParticleDa,
    /// 助詞「な」- 形容動詞連体形
    ParticleNa,
    /// 助詞「の」- 所有/連体修飾
    ParticleNo,
    /// 助詞「と」- 並列/共同
    ParticleTo,
    /// 助詞「が」- 主格
    ParticleGa,
    /// 副詞（実行修飾子）
    Adverb(String),
    
    // === Control Flow (Phase 4) ===
    /// if / もし
    KeywordIf,
    /// then / ならば
    KeywordThen,
    /// else / そうでなければ
    KeywordElse,
    /// end / おわり
    KeywordEnd,
    /// repeat / 繰り返す
    KeywordRepeat,
    /// times / 回
    KeywordTimes,
    
    // === Comparison ===
    /// equals / と等しい
    KeywordEquals,
    /// greater than / より大きい
    KeywordGreaterThan,
    /// less than / より小さい
    KeywordLessThan,
    
    // === English SVO keywords ===
    /// to (for "add X to Y")
    KeywordTo,
    /// let (for "let X = 10")
    KeywordLet,
    /// is (for "X is 10")
    KeywordIs,
    
    // === Phase 6: UI & Events ===
    /// 画面 / Screen (special noun)
    ScreenNoun,
    /// on / 〜とき
    KeywordOn,
    /// click / 押した
    KeywordClick,
    /// when / 〜とき
    KeywordWhen,
    
    // === Phase 10: Vector Graphics & UI ===
    /// の中 / Inside
    KeywordInside,
    /// 縦並び / Vertical
    KeywordVertical,
    /// 横並び / Horizontal
    KeywordHorizontal,

    // === Phase 11: Motion, State & Advanced Shaders ===
    /// 秒 / Seconds
    KeywordSeconds,
    /// 後 / After (time)
    KeywordAfter,
    /// かけて / Over (time)
    KeywordOver,
    /// 深くする / Deepen (Shadow)
    KeywordDeepen,
    /// にする / Change (Property)
    KeywordChange,
    /// 影 / Shadow
    KeywordShadow,
    /// マウス / Mouse
    KeywordMouse,
    /// 上 / Above
    KeywordAbove,
    /// 動かしたとき / Drag
    KeywordDrag,
    
    // === Eeyo: 空間・時間型 (Phase 13) ===
    /// 距離リテラル (10m, 5km)
    Distance { value: f64, unit: String },
    /// 時間リテラル (5分, 3秒, 1時間)
    Duration { value: f64, unit: String },
    
    // === Eeyo: 空間検索キーワード ===
    /// より近い
    KeywordNearer,
    /// より遠い
    KeywordFarther,
    /// 探す
    KeywordFind,
    /// 暇 / IDLE
    KeywordIdle,
    /// 発信する
    KeywordBroadcast,
    /// 通知する
    KeywordNotify,
    /// 人 (空間検索の対象)
    KeywordPerson,
    /// で (条件接続)
    ParticleDe,
    /// 間に / Between
    KeywordBetween,
    /// ある (存在)
    KeywordAre,
    
    // === Eeyo: 信頼スコア ===
    /// 徳
    KeywordToku,
    /// 加算する (徳スコア専用動詞)
    KeywordAccrue,
    
    // === AGN 2.0 (Social Layer) ===
    /// . (Dot) - プロパティアクセス
    Dot,
    /// ルール / Rule
    KeywordRule,
    /// アクション / Action
    KeywordAction,
    /// 結果 / Result
    KeywordResult,
    /// 増やす / Increase
    KeywordIncrease,
    /// 減らす / Decrease
    KeywordDecrease,
    /// 更新する / Update
    KeywordUpdate,
    /// 絆 / Bond
    KeywordBond,
    /// ランク / Rank
    KeywordRank,
    /// 付ける / Attach
    KeywordAttach,
    /// かつ / And
    KeywordAnd,
    
    // === Phase 15: Event-Driven Kizuna Logic ===
    /// から / From (Event Source)
    KeywordFrom,
    /// イベント / Event
    KeywordEvent,
    
    // Symbols
    /// {
    LBrace,
    /// }
    RBrace,
    /// (
    LParen,
    /// )
    RParen,
    /// ,
    Comma,
    
    /// 改行
    Newline,
    /// ファイル終端
    EOF,
}

/// 既知の日本語動詞リスト
const KNOWN_JP_VERBS: &[&str] = &[
    "足す", "引く", "掛ける", "割る", "表示する", "繰り返す",
    "要約する", "翻訳する", "読み込む", "つなぐ", // AI verbs & Asset Load & UI
    // Eeyo: 空間・通信動詞
    "探す", "発信する", "通知する", "加算する",
    // AGN 2.0: ソーシャル動詞
    "増やす", "減らす", "更新する", "付ける", "とする",
    "想像する", // Phase 11
    "get_bond", "set_status", // Phase 15 (Japanese context but func name likely reused or translated?)
    // 日本語エイリアスも検討: "絆を取得する", "ステータスを設定する"
];

/// 既知の英語動詞リスト
const KNOWN_EN_VERBS: &[&str] = &[
    "show", "add", "subtract", "multiply", "divide", "print",
    "summarize", "translate",  // AI verbs
    "get_bond", "set_status", // Phase 15
];

/// 特殊名詞（出力先など）
// const SPECIAL_NOUNS: &[&str] = &["画面", "Screen"];

/// 既知の副詞リスト
const KNOWN_ADVERBS: &[&str] = &["並列で", "async", "parallel"];

/// 英語キーワード
const ENGLISH_KEYWORDS: &[(&str, fn() -> Token)] = &[
    ("if", || Token::KeywordIf),
    ("then", || Token::KeywordThen),
    ("else", || Token::KeywordElse),
    ("end", || Token::KeywordEnd),
    ("repeat", || Token::KeywordRepeat),
    ("times", || Token::KeywordTimes),
    ("equals", || Token::KeywordEquals),
    ("to", || Token::KeywordTo),
    ("let", || Token::KeywordLet),
    ("is", || Token::KeywordIs),
    // Phase 6: UI & Events
    ("screen", || Token::ScreenNoun),
    ("on", || Token::KeywordOn),
    ("click", || Token::KeywordClick),
    ("when", || Token::KeywordWhen),
    
    // Phase 10
    ("inside", || Token::KeywordInside),
    ("vertical", || Token::KeywordVertical),
    ("horizontal", || Token::KeywordHorizontal),
    
    // AGN 2.0: Social Layer
    ("rule", || Token::KeywordRule),
    ("action", || Token::KeywordAction),
    ("result", || Token::KeywordResult),
    ("increase", || Token::KeywordIncrease),
    ("decrease", || Token::KeywordDecrease),
    ("update", || Token::KeywordUpdate),
    ("bond", || Token::KeywordBond),
    ("rank", || Token::KeywordRank),
    ("attach", || Token::KeywordAttach),
    ("and", || Token::KeywordAnd),
    // Phase 15
    ("from", || Token::KeywordFrom),
    ("event", || Token::KeywordEvent),
];

/// 日本語キーワード
const JAPANESE_KEYWORDS: &[(&str, fn() -> Token)] = &[
    ("もし", || Token::KeywordIf),
    ("ならば", || Token::KeywordThen),
    ("そうでなければ", || Token::KeywordElse),
    ("おわり", || Token::KeywordEnd),
    ("回", || Token::KeywordTimes),
    ("と等しい", || Token::KeywordEquals),
    ("より大きい", || Token::KeywordGreaterThan),
    ("より小さい", || Token::KeywordLessThan),
    // Phase 6: UI & Events
    ("画面", || Token::ScreenNoun),
    ("押したとき", || Token::KeywordClick),
    ("とき", || Token::KeywordWhen),
    
    // Phase 10
    ("の中", || Token::KeywordInside),
    ("縦並び", || Token::KeywordVertical),
    ("横並び", || Token::KeywordHorizontal),
    
    // Phase 11
    ("秒", || Token::KeywordSeconds),
    ("後", || Token::KeywordAfter),
    ("かけて", || Token::KeywordOver),
    ("深くする", || Token::KeywordDeepen),
    ("にする", || Token::KeywordChange),
    ("影", || Token::KeywordShadow),
    ("マウス", || Token::KeywordMouse),
    ("上", || Token::KeywordAbove),
    ("あるとき", || Token::KeywordWhen),
    ("動かしたとき", || Token::KeywordDrag),
    
    // Eeyo: 空間検索キーワード (Phase 13)
    ("より近い", || Token::KeywordNearer),
    ("より遠い", || Token::KeywordFarther),
    ("暇", || Token::KeywordIdle),
    ("人", || Token::KeywordPerson),
    ("徳", || Token::KeywordToku),
    ("で", || Token::ParticleDe),
    ("が", || Token::ParticleGa),
    
    // AGN 2.0: Social Layer
    ("ルール", || Token::KeywordRule),
    ("アクション", || Token::KeywordAction),
    ("結果", || Token::KeywordResult),
    ("増やす", || Token::KeywordIncrease),
    ("減らす", || Token::KeywordDecrease),
    ("更新する", || Token::KeywordUpdate),
    ("絆", || Token::KeywordBond),
    ("ランク", || Token::KeywordRank),
    ("付ける", || Token::KeywordAttach),
    ("がある", || Token::KeywordAre), // "絆がある" などの判定用
    ("にある", || Token::KeywordAre),
    ("かつ", || Token::KeywordAnd),
    // Phase 15
    ("から", || Token::KeywordFrom),
    ("イベント", || Token::KeywordEvent),
];

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_str(&self, len: usize) -> String {
        self.input[self.pos..].iter().take(len).collect()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_by(&mut self, n: usize) {
        self.pos += n;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c == ' ' || c == '\t' || c == '　' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let mut num_str = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_digit() || c == '.' {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let value = num_str.parse().unwrap_or(0.0);
        
        // Eeyo: 距離・時間リテラルの検出
        let remaining: String = self.input[self.pos..].iter().collect();
        
        // 距離単位: m, km
        if remaining.starts_with("km") {
            self.advance_by(2);
            return Token::Distance { value, unit: "km".to_string() };
        }
        if remaining.starts_with("m") && !remaining.starts_with("mm") {
            self.advance_by(1);
            return Token::Distance { value, unit: "m".to_string() };
        }
        
        // 時間単位: 分, 秒, 時間 (日本語)
        if remaining.starts_with("時間") {
            self.advance_by(2);
            return Token::Duration { value, unit: "時間".to_string() };
        }
        if remaining.starts_with("分後") {
            self.advance_by(2);
            return Token::Duration { value, unit: "分後".to_string() };
        }
        if remaining.starts_with("分") {
            self.advance_by(1);
            return Token::Duration { value, unit: "分".to_string() };
        }
        // Note: 秒 is handled by KeywordSeconds for animation syntax compatibility
        
        Token::Number(value)
    }

    fn read_string(&mut self) -> Token {
        self.advance(); // skip opening quote
        let mut s = String::new();
        while let Some(c) = self.current() {
            if c == '"' {
                self.advance(); // skip closing quote
                break;
            }
            s.push(c);
            self.advance();
        }
        Token::String(s)
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' || is_japanese_char(c) {
                let remaining: String = self.input[self.pos..].iter().collect();
                
                // 比較キーワードチェック (より大きい, より小さい)
                // These start with より which is not a particle, so check early
                if remaining.starts_with("より大きい") || remaining.starts_with("より小さい") {
                    break;
                }
                
                // 助詞チェック
                if remaining.starts_with("は") || remaining.starts_with("に") 
                   || remaining.starts_with("を") || remaining.starts_with("だ") 
                   || remaining.starts_with("な") || remaining.starts_with("の") {
                    break;
                }
                
                // 日本語キーワードチェック
                for (kw, _) in JAPANESE_KEYWORDS {
                    if remaining.starts_with(kw) && !ident.is_empty() {
                        return ident;
                    }
                }
                
                // 既知の日本語動詞チェック
                for verb in KNOWN_JP_VERBS {
                    if remaining.starts_with(verb) && !ident.is_empty() {
                        return ident;
                    }
                }
                
                // 既知の副詞チェック
                for adverb in KNOWN_ADVERBS {
                    if remaining.starts_with(adverb) && !ident.is_empty() {
                        return ident;
                    }
                }
                
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_english_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_alphabetic() || c == '_' {
                word.push(c);
                self.advance();
            } else {
                break;
            }
        }
        word
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            match self.current() {
                None => {
                    tokens.push(Token::EOF);
                    break;
                }
                Some('\n') | Some('\r') => {
                    tokens.push(Token::Newline);
                    self.advance();
                    if self.current() == Some('\n') {
                        self.advance();
                    }
                }
                Some('"') => {
                    tokens.push(self.read_string());
                }
                Some(c) if c.is_ascii_digit() => {
                    tokens.push(self.read_number());
                }
                Some(c) if c.is_ascii_alphabetic() => {
                    // 英語の単語を読む
                    let word = self.read_english_word();
                    let word_lower = word.to_lowercase();
                    
                    // 英語キーワードチェック
                    let mut matched = false;
                    for (kw, token_fn) in ENGLISH_KEYWORDS {
                        if word_lower == *kw {
                            tokens.push(token_fn());
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        continue;
                    }
                    
                    // 英語動詞チェック
                    for verb in KNOWN_EN_VERBS {
                        if word_lower == *verb {
                            tokens.push(Token::Verb(word_lower.clone()));
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        continue;
                    }
                    
                    // 英語副詞チェック
                    for adverb in KNOWN_ADVERBS {
                        if word_lower == *adverb {
                            tokens.push(Token::Adverb(word_lower.clone()));
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        continue;
                    }
                    
                    // それ以外は名詞
                    tokens.push(Token::Noun(word));
                }
                Some('{') => {
                    tokens.push(Token::LBrace);
                    self.advance();
                }
                Some('}') => {
                    tokens.push(Token::RBrace);
                    self.advance();
                }
                Some('(') => {
                    tokens.push(Token::LParen);
                    self.advance();
                }
                Some(')') => {
                    tokens.push(Token::RParen);
                    self.advance();
                }
                Some(',') | Some('、') => {
                    tokens.push(Token::Comma);
                    self.advance();
                }
                Some('>') => {
                    tokens.push(Token::KeywordGreaterThan);
                    self.advance();
                }
                Some('<') => {
                    tokens.push(Token::KeywordLessThan);
                    self.advance();
                }
                Some('=') => {
                    // Check for ==
                    if self.peek_str(1) == "=" {
                        tokens.push(Token::KeywordEquals);
                        self.advance_by(2);
                    } else {
                        // Single = is treated as Is/Assignment?
                        // Or just Equals?
                        // For now let's treat = as Equals for condition compatibility
                        tokens.push(Token::KeywordEquals);
                        self.advance();
                    }
                }
                Some('-') => {
                    // Check for negative number
                    let is_negative_number = if let Some(next) = self.input.get(self.pos + 1) {
                        next.is_ascii_digit()
                    } else {
                        false
                    };

                    if is_negative_number {
                        self.advance(); // consume '-'
                        let token = self.read_number();
                        let token = match token {
                            Token::Number(n) => Token::Number(-n),
                            Token::Distance { value, unit } => Token::Distance { value: -value, unit },
                            Token::Duration { value, unit } => Token::Duration { value: -value, unit },
                            _ => token,
                        };
                        tokens.push(token);
                    } else {
                        // Just a hyphen (maybe for separating words? or unknown)
                        // For now, skip it like before to avoid breaking other things
                        self.advance();
                    }
                }
                Some(_) => {
                    // コメント除去
                    if self.peek_str(2) == "//" {
                        while let Some(c) = self.current() {
                            if c == '\n' { break; }
                            self.advance();
                        }
                        continue;
                    }

                    // Dot (.) check
                    if self.current() == Some('.') {
                        // Check if next char is digit (fractional number)
                        let is_fraction = if let Some(next) = self.input.get(self.pos + 1) {
                            next.is_ascii_digit()
                        } else {
                            false
                        };

                        if is_fraction {
                            tokens.push(self.read_number());
                        } else {
                            tokens.push(Token::Dot);
                            self.advance();
                        }
                        continue;
                    }

                    // 日本語キーワードチェック (MUST come before particle check!)
                    // This ensures ならば is matched before なが matched as a particle
                    let mut matched_kw = false;
                    for (kw, token_fn) in JAPANESE_KEYWORDS {
                        let kw_len = kw.chars().count();
                        if self.peek_str(kw_len) == *kw {
                            tokens.push(token_fn());
                            self.advance_by(kw_len);
                            matched_kw = true;
                            break;
                        }
                    }
                    if matched_kw {
                        continue;
                    }

                    // 日本語助詞チェック (after keyword check)
                    if self.peek_str(1) == "は" {
                        tokens.push(Token::ParticleWa);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "に" {
                        tokens.push(Token::ParticleNi);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "を" {
                        tokens.push(Token::ParticleWo);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "だ" {
                        tokens.push(Token::ParticleDa);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "な" {
                        tokens.push(Token::ParticleNa);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "の" {
                        tokens.push(Token::ParticleNo);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "と" {
                        tokens.push(Token::ParticleTo);
                        self.advance();
                        continue;
                    }
                    if self.peek_str(1) == "が" {
                        tokens.push(Token::ParticleGa);
                        self.advance();
                        continue;
                    }

                    // 既知の副詞チェック
                    let mut matched_adv = false;
                    for adverb in KNOWN_ADVERBS {
                        let adv_len = adverb.chars().count();
                        if self.peek_str(adv_len) == *adverb {
                            tokens.push(Token::Adverb(adverb.to_string()));
                            self.advance_by(adv_len);
                            matched_adv = true;
                            break;
                        }
                    }
                    if matched_adv {
                        continue;
                    }

                    // 既知の日本語動詞チェック
                    let mut matched_verb = false;
                    for verb in KNOWN_JP_VERBS {
                        let verb_len = verb.chars().count();
                        if self.peek_str(verb_len) == *verb {
                            tokens.push(Token::Verb(verb.to_string()));
                            self.advance_by(verb_len);
                            matched_verb = true;
                            break;
                        }
                    }
                    if matched_verb {
                        continue;
                    }

                    // それ以外は名詞として読む
                    let ident = self.read_identifier();
                    if !ident.is_empty() {
                        tokens.push(Token::Noun(ident));
                    } else {
                        // 未知の文字はスキップ
                        self.advance();
                    }
                }
            }
        }

        tokens
    }
}

fn is_japanese_char(c: char) -> bool {
    let code = c as u32;
    (0x3040..=0x309F).contains(&code)  // ひらがな
        || (0x30A0..=0x30FF).contains(&code)  // カタカナ
        || (0x4E00..=0x9FFF).contains(&code)  // CJK統合漢字
        || (0x3400..=0x4DBF).contains(&code)  // CJK統合漢字拡張A
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assignment() {
        let mut lexer = Lexer::new("X は 10 だ");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Noun("X".to_string()));
        assert_eq!(tokens[1], Token::ParticleWa);
        assert_eq!(tokens[2], Token::Number(10.0));
        assert_eq!(tokens[3], Token::ParticleDa);
    }

    #[test]
    fn test_binary_op() {
        let mut lexer = Lexer::new("X に 5 を 足す");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Noun("X".to_string()));
        assert_eq!(tokens[1], Token::ParticleNi);
        assert_eq!(tokens[2], Token::Number(5.0));
        assert_eq!(tokens[3], Token::ParticleWo);
        assert_eq!(tokens[4], Token::Verb("足す".to_string()));
    }

    #[test]
    fn test_async_op() {
        let mut lexer = Lexer::new("X を 並列で 表示する");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Noun("X".to_string()));
        assert_eq!(tokens[1], Token::ParticleWo);
        assert_eq!(tokens[2], Token::Adverb("並列で".to_string()));
        assert_eq!(tokens[3], Token::Verb("表示する".to_string()));
    }

    // === Phase 4 Tests ===
    
    #[test]
    fn test_english_show() {
        let mut lexer = Lexer::new("show X");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Verb("show".to_string()));
        assert_eq!(tokens[1], Token::Noun("X".to_string()));
    }

    #[test]
    fn test_english_add_to() {
        let mut lexer = Lexer::new("add 5 to X");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Verb("add".to_string()));
        assert_eq!(tokens[1], Token::Number(5.0));
        assert_eq!(tokens[2], Token::KeywordTo);
        assert_eq!(tokens[3], Token::Noun("X".to_string()));
    }

    #[test]
    fn test_english_if_then_end() {
        let mut lexer = Lexer::new("if X equals 5 then show X end");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::KeywordIf);
        assert_eq!(tokens[1], Token::Noun("X".to_string()));
        assert_eq!(tokens[2], Token::KeywordEquals);
        assert_eq!(tokens[3], Token::Number(5.0));
        assert_eq!(tokens[4], Token::KeywordThen);
        assert_eq!(tokens[5], Token::Verb("show".to_string()));
        assert_eq!(tokens[6], Token::Noun("X".to_string()));
        assert_eq!(tokens[7], Token::KeywordEnd);
    }

    #[test]
    fn test_english_repeat() {
        let mut lexer = Lexer::new("repeat 10 times add 1 to X end");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::KeywordRepeat);
        assert_eq!(tokens[1], Token::Number(10.0));
        assert_eq!(tokens[2], Token::KeywordTimes);
    }

    #[test]
    fn test_japanese_if() {
        let mut lexer = Lexer::new("もし X と等しい 5 ならば 表示する X おわり");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::KeywordIf);
        assert_eq!(tokens[1], Token::Noun("X".to_string()));
        assert_eq!(tokens[2], Token::KeywordEquals);
    }

    #[test]
    fn test_japanese_repeat() {
        let mut lexer = Lexer::new("10 回 繰り返す X に 1 を 足す おわり");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Number(10.0));
        assert_eq!(tokens[1], Token::KeywordTimes);
        assert_eq!(tokens[2], Token::Verb("繰り返す".to_string()));
    }

    // === Eeyo: 空間・時間型テスト (Phase 13) ===

    #[test]
    fn test_distance_literal_meters() {
        let mut lexer = Lexer::new("10m");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Distance { value: 10.0, unit: "m".to_string() });
    }

    #[test]
    fn test_distance_literal_kilometers() {
        let mut lexer = Lexer::new("5km");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Distance { value: 5.0, unit: "km".to_string() });
    }

    #[test]
    fn test_duration_literal_minutes() {
        let mut lexer = Lexer::new("5分後");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Duration { value: 5.0, unit: "分後".to_string() });
    }

    #[test]
    fn test_spatial_search_keywords() {
        let mut lexer = Lexer::new("10m より近い 人 で 状態 が 暇 な 人 を 探す");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Distance { value: 10.0, unit: "m".to_string() });
        assert_eq!(tokens[1], Token::KeywordNearer);  // より近い
        assert_eq!(tokens[2], Token::KeywordPerson);  // 人
        assert_eq!(tokens[3], Token::ParticleDe);     // で
    }

    // === AGN 2.0 (Social Layer) Tests ===

    #[test]
    fn test_dot_access() {
        let mut lexer = Lexer::new("User.Toku");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Noun("User".to_string()));
        assert_eq!(tokens[1], Token::Dot);
        assert_eq!(tokens[2], Token::Noun("Toku".to_string()));
    }

    #[test]
    fn test_dot_number_disambiguation() {
        // .5 は数値、.Toku lambda は Dot + Noun
        let mut lexer = Lexer::new("0.5 .Toku");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Number(0.5));
        assert_eq!(tokens[1], Token::Dot);
        assert_eq!(tokens[2], Token::Noun("Toku".to_string()));
    }

    #[test]
    fn test_rule_definition() {
        let mut lexer = Lexer::new("ルール MyRule");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::KeywordRule);
        assert_eq!(tokens[1], Token::Noun("MyRule".to_string()));
    }

    #[test]
    fn test_action_verbs() {
        let mut lexer = Lexer::new("徳 を 増やす ランク を 更新する");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::KeywordToku);
        assert_eq!(tokens[1], Token::ParticleWo);
        assert_eq!(tokens[2], Token::KeywordIncrease);
        assert_eq!(tokens[3], Token::KeywordRank);
        assert_eq!(tokens[4], Token::ParticleWo);
        assert_eq!(tokens[5], Token::KeywordUpdate);
    }
}
