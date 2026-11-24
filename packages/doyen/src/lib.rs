use napi_derive::napi;

#[napi]
pub fn raiz() -> String {
    "Olá, mundo!".to_owned()
}

#[napi]
pub fn saudacao(nome: String) -> String {
    format!("Olá, {nome}!")
}

#[cfg(test)]
mod tests {
    use super::{raiz, saudacao};

    #[test]
    fn root_response_is_preserved() {
        assert_eq!(raiz(), "Olá, mundo!");
    }

    #[test]
    fn greeting_response_is_preserved() {
        assert_eq!(saudacao("Maria".to_owned()), "Olá, Maria!");
        assert_eq!(saudacao("João".to_owned()), "Olá, João!");
    }
}
