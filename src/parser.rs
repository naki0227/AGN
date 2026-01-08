//! AGN Parser - 構文解析器
//! 日本語SOV構文と英語SVO構文の両方を解析してASTを生成する

use crate::lexer::Token;

/// 式（値を表す）
#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Variable(String),
}

/// 条件式
#[derive(Debug, Clone)]
pub enum Condition {
    Equals(Expr, Expr),
    GreaterThan(Expr, Expr),
    LessThan(Expr, Expr),
}

/// 文（実行単位）
#[derive(Debug, Clone)]
pub enum Statement {
    /// 代入文: [名詞] は [値] だ / let X = 10
    Assignment { name: String, value: Expr },
    /// アセットロード: [ターゲット] は [パス] を 読み込む
    LoadAsset {
        target: String,
        path: Expr,
    },
    /// UIコンポーネント定義: [名前] は [スタイル] な [コンポーネント] だ
    ComponentDefine {
        target: String,
        style: String,
        component: String,
    },
    /// 二項演算: [名詞] に [値] を [動詞] / add [値] to [名詞]
    BinaryOp { target: String, operand: Expr, verb: String },
    /// 単項関数: [値] を [動詞] / show [値]
    UnaryOp { operand: Expr, verb: String },
    /// 非同期実行: [値] を 並列で [動詞]
    AsyncOp { operand: Expr, verb: String },
    /// 条件分岐: if [条件] then [処理] end
    IfStatement {
        condition: Condition,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    /// ループ: repeat [回数] times [処理] end / [回数] 回 繰り返す [処理] おわり
    RepeatStatement {
        count: Expr,
        body: Vec<Statement>,
    },
    /// AI操作: [結果] は [入力] を [AI動詞] / Result is summarize Input
    AiOp {
        result: String,
        input: Expr,
        verb: String,
        options: Option<String>,
    },
    /// 画面出力: [値] を 画面 に 表示する / show [値] to Screen
    ScreenOp {
        operand: Expr,
    },
    /// イベントハンドラ: on [名詞] click ... end / [名詞] を 押したとき ... おわり
    EventHandler {
        target: String,
        event: String,
        body: Vec<Statement>,
    },
    
    // === Phase 10: Vector Graphics & UI ===
    /// ブロック: [名詞] の 中 に ... おわり
    Block {
        target: String,
        body: Vec<Statement>,
    },
    /// レイアウト: [リスト] を [方向] に 置く
    Layout {
        target: String, // "これら" (These) usually, or parent implicit? 
                        // Actually logic: "これら を 縦並び に 置く" 
                        // "これら" refers to children of current block?
                        // But parser doesn't know parent.
                        // So we emit a statement "SetLayout { direction }"
        direction: LayoutDirection,
    },
    
    // === Phase 11: Motion, State & Advanced Shaders ===
    /// アニメーション: [時間] かけて [プロパティ] を [値] にする
    Animate {
        duration: f64,
        property: String, // e.g. "影", "背景"
        target_value: Expr, // e.g. "深くする" (enum?), or Color Value
        // Since "深くする" is a keyword/concept, maybe target_value is Expr?
        // "背景 を 水色 に する" -> property="背景", target="水色"
        // "影 を 深くする" -> property="影", target="深くする" (Special value?)
        // Let's use Expr::Variable("深くする") or similar.
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

/// プログラム全体
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn peek(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current(), Token::Newline) {
            self.advance();
        }
    }

    fn parse_expr(&self, token: &Token) -> Option<Expr> {
        match token {
            Token::Number(n) => Some(Expr::Number(*n)),
            Token::String(s) => Some(Expr::String(s.clone())),
            Token::Noun(name) => Some(Expr::Variable(name.clone())),
            _ => None,
        }
    }

    fn current_to_expr(&mut self) -> Result<Expr, String> {
        let token = self.current().clone();
        let expr = self.parse_expr(&token)
            .ok_or_else(|| format!("Expected expression, got {:?}", token))?;
        self.advance();
        Ok(expr)
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        loop {
            self.skip_newlines();

            if matches!(self.current(), Token::EOF) {
                break;
            }

            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.skip_newlines();
        
        // === English SVO Patterns ===
        
        // English: show X to Screen (check before regular show)
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") 
            && self.look_for_screen_target() {
            return self.parse_show_to_screen();
        }
        
        // English: show X / print X
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") {
            return self.parse_english_show();
        }
        
        // English: summarize X / translate X (AI verbs)
        if matches!(self.current(), Token::Verb(v) if v == "summarize" || v == "translate") {
            return self.parse_english_ai_verb();
        }
        
        // English: add X to Y
        if matches!(self.current(), Token::Verb(v) if v == "add" || v == "subtract" || v == "multiply" || v == "divide") {
            return self.parse_english_binary_op();
        }
        
        // English: if X equals Y then ... end
        if matches!(self.current(), Token::KeywordIf) {
            return self.parse_if_statement();
        }
        
        // English: repeat N times ... end
        if matches!(self.current(), Token::KeywordRepeat) {
            return self.parse_english_repeat();
        }
        
        // English: let X = 10 (optional)
        if matches!(self.current(), Token::KeywordLet) {
            return self.parse_english_let();
        }
        
        // English: X is 10 (like Japanese X は 10 だ)
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::KeywordIs) {
            return self.parse_english_is_assignment();
        }
        
        // === Japanese SOV Patterns ===
        
        // 日本語: [名詞] は [値] だ / [パス] を 読み込む / [スタイル] な [コン] だ
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::ParticleWa) {
            return self.parse_assignment();
        }

        // 日本語: [名詞] に [値] を [動詞]
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::ParticleNi) {
            return self.parse_binary_op();
        }

        // Phase 10: [名詞] を [方向] に 置く
        // Check if Noun leads to Layout pattern: Noun + Wo + Direction
        // Must check BEFORE UnaryOrAsync because both start with Noun + Wo
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && self.is_direction_token(self.peek(2)) {
             return self.parse_layout();
        }

        // Phase 12: [名詞] を 押したとき / 動かしたとき
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && (matches!(self.peek(2), Token::KeywordClick | Token::KeywordDrag)) {
             return self.parse_object_event();
        }
        
        if matches!(self.peek(1), Token::ParticleWo) {
             // println!("Debug: Noun+Wo detected. peek2={:?}", self.peek(2));
        }

        // 日本語: [対象] を [ターゲット] に [動詞] (逆順: O を T に V)
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && (matches!(self.peek(2), Token::Noun(_) | Token::ScreenNoun) && matches!(self.peek(3), Token::ParticleNi)) {
             return self.parse_binary_op_reverse();
        }

        // 日本語: [値] を (並列で)? [動詞]
        if matches!(self.peek(1), Token::ParticleWo) {
            return self.parse_unary_or_async();
        }
        
        // 日本語: [数値] 回 繰り返す ... おわり
        if matches!(self.current(), Token::Number(_)) && matches!(self.peek(1), Token::KeywordTimes) {
            return self.parse_japanese_repeat();
        }
        
        // 日本語: もし ... ならば ... おわり
        if matches!(self.current(), Token::KeywordIf) {
            return self.parse_if_statement();
        }
        
        // === Phase 6: UI Patterns ===
        
        if matches!(self.current(), Token::KeywordOn) {
            return self.parse_on_event();
        }
        
        // Phase 10: [名詞] の 中 に ... おわり
        // Check for Noun followed by ParticleNo + KeywordInside + ParticleNi?
        // Actually Lexer returns KeywordInside for "の中".
        // Pattern: [Noun] [KeywordInside] [ParticleNi]
        // Phase 10: [名詞] の 中 に ... おわり
        // Phase 10: [名詞] の 中 に ... おわり
        // Support both "の中" (KeywordInside) and "の" "中" (ParticleNo + Noun("中"))
        if matches!(self.current(), Token::Noun(_)) {
            if matches!(self.peek(1), Token::KeywordInside) {
                return self.parse_block();
            }
            if matches!(self.peek(1), Token::ParticleNo) 
               && matches!(self.peek(2), Token::Noun(n) if n == "中") {
                return self.parse_block();
            }
        }

        // Phase 11: [数値] 秒 かけて [プロパティ] を [値] にする / 深くする
        if matches!(self.current(), Token::Number(_)) && matches!(self.peek(1), Token::KeywordSeconds) {
             return self.parse_animate();
        }
        
        // Phase 11: マウス が 上 に あるとき
        if matches!(self.current(), Token::KeywordMouse) {
             return self.parse_mouse_event();
        }
        

        
        // English: show X to Screen
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") 
            && self.look_for_screen_target() {
            return self.parse_show_to_screen();
        }

        Err(format!("Unexpected token: {:?}", self.current()))
    }

    // === English SVO Parsers ===
    
    fn parse_english_show(&mut self) -> Result<Statement, String> {
        // show X / print X
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance(); // skip verb
        
        let operand = self.current_to_expr()?;
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "show" | "print" => "表示する".to_string(),
            _ => verb,
        };
        
        Ok(Statement::UnaryOp { operand, verb: normalized_verb })
    }
    
    fn parse_english_ai_verb(&mut self) -> Result<Statement, String> {
        // summarize X / translate X
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected AI verb".to_string()),
        };
        self.advance(); // skip verb
        
        let input = self.current_to_expr()?;
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "summarize" => "要約する".to_string(),
            "translate" => "翻訳する".to_string(),
            _ => verb,
        };
        
        // This returns a value, so we need an assignment context
        // For now, treat as UnaryOp that returns a value
        Ok(Statement::UnaryOp { operand: input, verb: normalized_verb })
    }
    
    fn parse_english_binary_op(&mut self) -> Result<Statement, String> {
        // add X to Y / subtract X from Y
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance(); // skip verb
        
        let operand = self.current_to_expr()?;
        
        // Expect "to"
        if !matches!(self.current(), Token::KeywordTo) {
            return Err("Expected 'to'".to_string());
        }
        self.advance(); // skip to
        
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected variable name".to_string()),
        };
        self.advance(); // skip target
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "add" => "足す".to_string(),
            "subtract" => "引く".to_string(),
            "multiply" => "掛ける".to_string(),
            "divide" => "割る".to_string(),
            _ => verb,
        };
        
        Ok(Statement::BinaryOp { target, operand, verb: normalized_verb })
    }
    
    fn parse_english_let(&mut self) -> Result<Statement, String> {
        // let X = 10
        self.advance(); // skip let
        
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected variable name".to_string()),
        };
        self.advance(); // skip name
        
        // Expect "is" or "="
        if matches!(self.current(), Token::KeywordIs) {
            self.advance();
        }
        
        let value = self.current_to_expr()?;
        
        Ok(Statement::Assignment { name, value })
    }
    
    fn parse_english_is_assignment(&mut self) -> Result<Statement, String> {
        // X is 10 (like Japanese X は 10 だ)
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected variable name".to_string()),
        };
        self.advance(); // skip name
        
        // Expect "is"
        if !matches!(self.current(), Token::KeywordIs) {
            return Err("Expected 'is'".to_string());
        }
        self.advance(); // skip is
        
        let value = self.current_to_expr()?;
        
        Ok(Statement::Assignment { name, value })
    }
    
    fn parse_english_repeat(&mut self) -> Result<Statement, String> {
        // repeat N times ... end
        self.advance(); // skip repeat
        
        let count = self.current_to_expr()?;
        
        // Expect "times"
        if !matches!(self.current(), Token::KeywordTimes) {
            return Err("Expected 'times'".to_string());
        }
        self.advance(); // skip times
        
        // Parse body until "end"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::RepeatStatement { count, body })
    }
    
    fn parse_if_statement(&mut self) -> Result<Statement, String> {
        // if X equals Y then ... end
        // もし X と等しい Y ならば ... おわり
        self.advance(); // skip if / もし
        
        // Parse condition
        let left = self.current_to_expr()?;
        
        // Expect comparison operator
        let condition = match self.current() {
            Token::KeywordEquals => {
                self.advance();
                let right = self.current_to_expr()?;
                Condition::Equals(left, right)
            }
            Token::KeywordGreaterThan => {
                self.advance();
                let right = self.current_to_expr()?;
                Condition::GreaterThan(left, right)
            }
            Token::KeywordLessThan => {
                self.advance();
                let right = self.current_to_expr()?;
                Condition::LessThan(left, right)
            }
            _ => return Err(format!("Expected comparison operator, got {:?}", self.current())),
        };
        
        // Expect "then" / "ならば"
        if !matches!(self.current(), Token::KeywordThen) {
            return Err("Expected 'then' or 'ならば'".to_string());
        }
        self.advance(); // skip then
        
        // Parse then block
        let then_block = self.parse_block_until_end_or_else()?;
        
        // Check for else
        let else_block = if matches!(self.current(), Token::KeywordElse) {
            self.advance(); // skip else
            Some(self.parse_block_until_end()?)
        } else {
            None
        };
        
        // "end" is consumed by parse_block_until_end
        
        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
        })
    }
    
    fn parse_japanese_repeat(&mut self) -> Result<Statement, String> {
        // N 回 繰り返す ... おわり
        let count = self.current_to_expr()?;
        
        // Expect "回"
        if !matches!(self.current(), Token::KeywordTimes) {
            return Err("Expected '回'".to_string());
        }
        self.advance(); // skip 回
        
        // Expect "繰り返す"
        if !matches!(self.current(), Token::Verb(v) if v == "繰り返す") {
            return Err("Expected '繰り返す'".to_string());
        }
        self.advance(); // skip 繰り返す
        
        // Parse body until "おわり"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::RepeatStatement { count, body })
    }
    
    // === Phase 6: UI Parsing ===
    
    fn is_direction_token(&self, token: &Token) -> bool {
        matches!(token, Token::KeywordVertical | Token::KeywordHorizontal) ||
        matches!(token, Token::Noun(n) if n == "縦並び" || n == "横並び")
    }
    
    /// Screen出力を先読みでチェック
    fn look_for_screen_target(&self) -> bool {
        // show X to Screen パターンを検出
        for i in 0..5 {
            if matches!(self.peek(i), Token::ScreenNoun) {
                return true;
            }
        }
        false
    }
    
    fn parse_on_event(&mut self) -> Result<Statement, String> {
        // on Button click ... end
        self.advance(); // skip on
        
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected element name".to_string()),
        };
        self.advance(); // skip target
        
        // Expect "click"
        let event = if matches!(self.current(), Token::KeywordClick) {
            self.advance();
            "click".to_string()
        } else {
            return Err("Expected 'click'".to_string());
        };
        
        // Parse body until "end"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::EventHandler { target, event, body })
    }
    
    fn parse_show_to_screen(&mut self) -> Result<Statement, String> {
        // show X to Screen
        self.advance(); // skip show/print
        
        let operand = self.current_to_expr()?;
        
        // Expect "to"
        if !matches!(self.current(), Token::KeywordTo) {
            return Err("Expected 'to'".to_string());
        }
        self.advance(); // skip to
        
        // Expect "Screen"
        if !matches!(self.current(), Token::ScreenNoun) {
            return Err("Expected 'Screen'".to_string());
        }
        self.advance(); // skip Screen
        
        Ok(Statement::ScreenOp { operand })
    }
    
    fn parse_block_until_end(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        loop {
            self.skip_newlines();
            
            if matches!(self.current(), Token::KeywordEnd | Token::EOF) {
                if matches!(self.current(), Token::KeywordEnd) {
                    self.advance(); // skip end
                }
                break;
            }
            
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        
        Ok(statements)
    }
    
    fn parse_block_until_end_or_else(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        loop {
            self.skip_newlines();
            
            if matches!(self.current(), Token::KeywordEnd | Token::KeywordElse | Token::EOF) {
                if matches!(self.current(), Token::KeywordEnd) {
                    self.advance(); // skip end
                }
                break;
            }
            
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        
        Ok(statements)
    }

    // === Phase 10 Parsers ===

    fn parse_block(&mut self) -> Result<Statement, String> {
        // [名詞] の 中 に ... おわり
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance(); // skip noun
        
        // Expect "の中" (KeywordInside) OR "の" "中"
        if matches!(self.current(), Token::KeywordInside) {
            self.advance();
        } else if matches!(self.current(), Token::ParticleNo) {
            self.advance();
            // Expect "中"
            match self.current() {
                Token::Noun(n) if n == "中" => {
                    self.advance();
                }
                _ => return Err("Expected '中' after 'の'".to_string()),
            }
        } else {
            return Err("Expected 'の中' or 'の 中'".to_string());
        }
        
        // Expect "に" (ParticleNi)
        if !matches!(self.current(), Token::ParticleNi) {
             return Err("Expected 'に'".to_string());
        }
        self.advance();
        
        // Parse body
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::Block { target, body })
    }

    fn parse_layout(&mut self) -> Result<Statement, String> {
        // [名詞] を [方向] に 置く
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance(); // skip target
        
        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();
        
        let direction = match self.current() {
            Token::KeywordVertical => LayoutDirection::Vertical,
            Token::KeywordHorizontal => LayoutDirection::Horizontal,
            Token::Noun(n) if n == "縦並び" => LayoutDirection::Vertical, // Fallback if lexer didn't catch
            Token::Noun(n) if n == "横並び" => LayoutDirection::Horizontal,
            _ => return Err("Expected '縦並び' or '横並び'".to_string()),
        };
        self.advance();
        
        // Expect "に" (ParticleNi)
        if matches!(self.current(), Token::ParticleNi) {
             self.advance();
        }
        
        // For robustness, handle both.
        match self.current() {
            Token::Verb(v) if v == "置く" => self.advance(),
            Token::Noun(n) if n == "置く" => self.advance(),
            _ => return Err("Expected '置く'".to_string()),
        };
        
        Ok(Statement::Layout { target, direction })
    }

    // === Phase 11: Animation & Event Parsers ===

    fn parse_animate(&mut self) -> Result<Statement, String> {
        // [数値] 秒 かけて [プロパティ] を [値] にする / 深くする
        let duration = match self.current() {
            Token::Number(n) => *n,
            _ => return Err("Expected duration number".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::KeywordSeconds) {
            return Err("Expected '秒'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::KeywordOver) {
            return Err("Expected 'かけて'".to_string());
        }
        self.advance();

        // Property: "影" or "背景" (Noun)
        let property = match self.current() {
             Token::KeywordShadow => "shadow".to_string(), // "影"
             Token::Noun(n) => n.clone(),
             _ => return Err("Expected property name (影 or noun)".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();

        // Target Value: "深くする" (Deepen) or "[Value] にする"
        let target_value = if matches!(self.current(), Token::KeywordDeepen) {
             self.advance();
             Expr::String("deepen".to_string()) // Special value
        } else {
             // [Value] にする
             let val = self.current_to_expr()?;
             
             if matches!(self.current(), Token::ParticleNi) {
                 self.advance();
             }
             
             if !matches!(self.current(), Token::KeywordChange) {
                 // Check verb "する"?
                 match self.current() {
                     Token::Verb(v) if v == "する" => { self.advance(); },
                     Token::Noun(n) if n == "する" => { self.advance(); },
                     Token::KeywordChange => { self.advance(); },
                     Token::Noun(n) if n == "にする" => { self.advance(); },
                     _ => return Err("Expected 'にする' or 'する'".to_string()),
                 }
             } else {
                 self.advance(); // skip 'change'
             }
             val
        };

        Ok(Statement::Animate { duration, property, target_value })
    }

    fn parse_mouse_event(&mut self) -> Result<Statement, String> {
        // マウス が 上 に あるとき
        // Token::KeywordMouse has been consumed? No, parser dispatcher peeks.
        self.advance(); // skip Mouse

        // Skip optional particle "ga"
        if matches!(self.current(), Token::Noun(n) if n == "が") {
            self.advance();
        }

        if !matches!(self.current(), Token::KeywordAbove) { // "上"
             return Err("Expected '上'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::ParticleNi) { // "に"
             return Err("Expected 'に'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::KeywordWhen) { // "あるとき"
             return Err("Expected 'あるとき'".to_string());
        }
        self.advance();

        let body = self.parse_block_until_end()?;

        Ok(Statement::EventHandler { 
            target: "self".to_string(), // Implicit target (current component)
            event: "hover".to_string(), 
            body 
        })
    }

    fn parse_object_event(&mut self) -> Result<Statement, String> {
        // [名詞] を 押したとき / 動かしたとき
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected target noun".to_string()),
        };
        self.advance(); // skip target
        
        // Expect "を" - checked by dispatcher
        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();
        
        let event = match self.current() {
            Token::KeywordClick => "click".to_string(),
            Token::KeywordDrag => "drag".to_string(),
            _ => return Err("Expected event type (押したとき/動かしたとき)".to_string()),
        };
        self.advance();
        
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::EventHandler { target, event, body })
    }

    // === Japanese SOV Parsers (existing) ===
    
    fn parse_assignment(&mut self) -> Result<Statement, String> {
        // [名詞] は ...
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::ParticleWa) {
            return Err("Expected 'は'".to_string());
        }
        self.advance();

        let expr = self.current_to_expr()?;

        match self.current() {
            Token::ParticleDa => {
                // [値] だ
                self.advance();
                Ok(Statement::Assignment { name, value: expr })
            }
            Token::ParticleWo => {
                // [パス] を 読み込む
                self.advance();
                match self.current() {
                    Token::Verb(v) if v == "読み込む" => {
                        self.advance();
                        Ok(Statement::LoadAsset { target: name, path: expr })
                    }
                    _ => Err("Expected '読み込む' after 'を' in assignment context".to_string())
                }
            }
            Token::ParticleNa => {
                // [スタイル] な [コンポーネント] だ
                self.advance(); // skip Na
                // Expect Component Noun
                let component = match self.current() {
                     Token::Noun(n) => n.clone(),
                     _ => return Err("Expected component noun".to_string()),
                };
                self.advance();
                
                if !matches!(self.current(), Token::ParticleDa) {
                     return Err("Expected 'だ'".to_string());
                }
                self.advance();
                
                // Extract style from expr
                let style = match expr {
                    Expr::Variable(s) => s,
                    _ => return Err("Expected style variable".to_string()),
                };
                Ok(Statement::ComponentDefine { target: name, style, component })
            }
            _ => Err("Expected 'だ', 'を', or 'な'".to_string())
        }
    }

    fn parse_binary_op(&mut self) -> Result<Statement, String> {
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::ParticleNi) {
            return Err("Expected 'に'".to_string());
        }
        self.advance();

        let operand = self.current_to_expr()?;

        if !matches!(self.current(), Token::ParticleWo) {
            return Err("Expected 'を'".to_string());
        }
        self.advance();

        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();

        Ok(Statement::BinaryOp { target, operand, verb })
    }

    fn parse_binary_op_reverse(&mut self) -> Result<Statement, String> {
        // [operand] を [target] に [verb]
        let operand = self.current_to_expr()?;
        
        // Skip "を"
        self.advance();
        
        // Get target
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            Token::ScreenNoun => "Screen".to_string(),
            _ => return Err("Expected target noun".to_string()),
        };
        self.advance();
        
        // Skip "に"
        if !matches!(self.current(), Token::ParticleNi) {
            return Err("Expected 'に'".to_string());
        }
        self.advance();
        
        // Get verb
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();
        
        Ok(Statement::BinaryOp { target, operand, verb })
    }

    fn parse_unary_or_async(&mut self) -> Result<Statement, String> {
        let operand = self.current_to_expr()?;

        if !matches!(self.current(), Token::ParticleWo) {
            return Err("Expected 'を'".to_string());
        }
        self.advance();

        let is_async = matches!(self.current(), Token::Adverb(a) if a == "並列で");
        if is_async {
            self.advance();
        }

        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();

        if is_async {
            Ok(Statement::AsyncOp { operand, verb })
        } else {
            Ok(Statement::UnaryOp { operand, verb })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_assignment() {
        let mut lexer = Lexer::new("X は 10 だ");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { name, value } => {
                assert_eq!(name, "X");
                match value {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected assignment"),
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let mut lexer = Lexer::new("X に 5 を 足す");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::BinaryOp { target, operand, verb } => {
                assert_eq!(target, "X");
                assert_eq!(verb, "足す");
                match operand {
                    Expr::Number(n) => assert_eq!(*n, 5.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected binary op"),
        }
    }

    #[test]
    fn test_parse_async_op() {
        let mut lexer = Lexer::new("X を 並列で 表示する");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::AsyncOp { operand, verb } => {
                assert_eq!(verb, "表示する");
                match operand {
                    Expr::Variable(name) => assert_eq!(name, "X"),
                    _ => panic!("Expected variable"),
                }
            }
            _ => panic!("Expected async op"),
        }
    }

    // === Phase 4 Tests ===
    
    #[test]
    fn test_parse_english_show() {
        let mut lexer = Lexer::new("show X");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::UnaryOp { operand, verb } => {
                assert_eq!(verb, "表示する");
                match operand {
                    Expr::Variable(name) => assert_eq!(name, "X"),
                    _ => panic!("Expected variable"),
                }
            }
            _ => panic!("Expected unary op"),
        }
    }

    #[test]
    fn test_parse_english_add_to() {
        let mut lexer = Lexer::new("add 5 to X");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::BinaryOp { target, operand, verb } => {
                assert_eq!(target, "X");
                assert_eq!(verb, "足す");
                match operand {
                    Expr::Number(n) => assert_eq!(*n, 5.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected binary op"),
        }
    }

    #[test]
    fn test_parse_english_if() {
        let mut lexer = Lexer::new("if X equals 5 then show X end");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::IfStatement { condition, then_block, else_block } => {
                match condition {
                    Condition::Equals(_, _) => {}
                    _ => panic!("Expected equals condition"),
                }
                assert_eq!(then_block.len(), 1);
                assert!(else_block.is_none());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_parse_english_repeat() {
        let mut lexer = Lexer::new("repeat 10 times add 1 to X end");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::RepeatStatement { count, body } => {
                match count {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected repeat statement"),
        }
    }

    #[test]
    fn test_parse_let() {
        let mut lexer = Lexer::new("let X is 10");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { name, value } => {
                assert_eq!(name, "X");
                match value {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected assignment"),
        }
    }
}
