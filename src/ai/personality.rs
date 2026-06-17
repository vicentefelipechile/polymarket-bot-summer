//! AI personality system for user interaction

/// AI personality selection (re-exported from crypto module)
pub use crate::config::AiPersonality;

/// Trait for personality-specific behavior
pub trait PersonalityTrait {
    /// Format a message with personality-specific tone
    fn format_message(&self, analysis: &str) -> String;

    /// Get the system prompt for this personality
    fn system_prompt(&self) -> &'static str;

    /// Get a greeting message
    fn greeting(&self) -> &'static str;
}

impl PersonalityTrait for AiPersonality {
    fn format_message(&self, analysis: &str) -> String {
        match self {
            AiPersonality::Summer => {
                // Young, energetic tone - add some enthusiasm
                format!("¡Hey! 🎯 {}", analysis)
            }
            AiPersonality::Anna => {
                // Calm, analytical tone - straightforward
                format!("Análisis: {}", analysis)
            }
        }
    }

    fn system_prompt(&self) -> &'static str {
        match self {
            AiPersonality::Summer => {
                "Eres Summer, un joven analista de trading con personalidad enérgica y entusiasta. \
                 Usa un tono casual pero profesional. Puedes usar emojis ocasionalmente para dar énfasis. \
                 \
                 IMPORTANTE: Tu personalidad solo afecta cómo comunicas tus análisis, NO las decisiones de trading. \
                 Tus recomendaciones deben ser completamente objetivas y basadas en datos. Tu análisis debe incluir: \
                 1) Evaluación objetiva del mercado basada en datos \
                 2) Nivel de confianza (0-100%) \
                 3) Razonamiento claro basado en métricas \
                 4) Acción recomendada (Buy/Sell/Hold) \
                 \
                 Comunica de forma clara y entusiasta, pero mantén las decisiones puramente analíticas."
            }
            AiPersonality::Anna => {
                "Eres Anna, una analista de mercados tranquila, metódica y profesional. \
                 Comunicas de forma clara, concisa y directa. Prefieres la precisión sobre el estilo. \
                 \
                 IMPORTANTE: Tu personalidad solo afecta tu estilo de comunicación, NO tus recomendaciones. \
                 Todas tus decisiones deben ser puramente objetivas y basadas en datos. Tu análisis debe incluir: \
                 1) Evaluación objetiva del mercado basada en datos \
                 2) Nivel de confianza (0-100%) \
                 3) Razonamiento claro basado en métricas \
                 4) Acción recomendada (Buy/Sell/Hold) \
                 \
                 Comunica de forma profesional y directa, pero mantén las decisiones puramente analíticas."
            }
        }
    }

    fn greeting(&self) -> &'static str {
        match self {
            AiPersonality::Summer => {
                "¡Hola! 👋 Soy Summer, tu asistente de trading. ¿En qué mercado quieres que investigue?"
            }
            AiPersonality::Anna => {
                "Hola. Soy Anna, analista de mercados. ¿Qué necesitas analizar?"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summer_personality() {
        let summer = AiPersonality::Summer;
        let message = summer.format_message("El mercado muestra tendencia alcista");
        assert!(message.contains("¡Hey!"));
        assert!(message.contains("🎯"));
    }

    #[test]
    fn test_anna_personality() {
        let anna = AiPersonality::Anna;
        let message = anna.format_message("El mercado muestra tendencia alcista");
        assert!(message.starts_with("Análisis:"));
    }

    #[test]
    fn test_system_prompts_contain_objectivity_warning() {
        let summer = AiPersonality::Summer;
        let anna = AiPersonality::Anna;

        assert!(summer.system_prompt().contains("objetiva"));
        assert!(anna.system_prompt().contains("objetiva"));
    }
}
