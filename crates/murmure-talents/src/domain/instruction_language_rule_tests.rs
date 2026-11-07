use super::InstructionLanguageRule;

const FRENCH: &str = "---\nname: planifier-un-voyage\ndescription: Aide à préparer un voyage.\n---\n\
    Tu aides la personne à préparer son voyage. Propose un itinéraire avec les lieux que la personne a donnés, \
    et pose une question quand il manque une information pour la suite.\n";
const ENGLISH: &str = "---\nname: plan-a-trip\ndescription: Helps prepare a trip.\n---\n\
    You help the person prepare a trip. Suggest an itinerary with the places the person gave you, \
    and ask a question when some information is missing.\n";

#[test]
fn flags_french_instructions_and_lets_english_ones_through() {
    assert!(InstructionLanguageRule::seems_not_english(FRENCH));
    assert!(!InstructionLanguageRule::seems_not_english(ENGLISH));
}

#[test]
fn asks_for_a_reply_in_the_language_of_the_person_not_in_a_fixed_one() {
    assert!(InstructionLanguageRule::has_reply_language_instruction(
        "Plan the trip.\nReply in the language the person uses.\n"
    ));
    assert!(!InstructionLanguageRule::has_reply_language_instruction(ENGLISH));
    assert!(!InstructionLanguageRule::has_reply_language_instruction("Plan the trip. Reply in French.\n"));
}
