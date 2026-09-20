//! Chart-of-accounts seeding: every newly-registered tenant must get the
//! full plan de cuentas, including the Patrimonio (equity) accounts needed
//! for a balance sheet to ever balance (Activo = Pasivo + Patrimonio).

mod common;

use common::*;

#[tokio::test]
async fn new_tenant_is_seeded_with_patrimonio_accounts() {
    let session = register_tenant().await;

    let cuentas = session.get("/v1/contabilidad/cuentas").await;
    let cuentas = cuentas.as_array().expect("GET /v1/contabilidad/cuentas must return an array");

    let capital = cuentas.iter().find(|c| c["codigo"] == "3100").expect("3100 Capital Social must exist");
    assert_eq!(capital["tipo"], "PATRIMONIO", "3100 must be tipo PATRIMONIO");
    assert_eq!(capital["naturaleza"], "ACREEDORA", "3100 must be naturaleza ACREEDORA");

    let resultados = cuentas.iter().find(|c| c["codigo"] == "3200").expect("3200 Resultados del Ejercicio must exist");
    assert_eq!(resultados["tipo"], "PATRIMONIO", "3200 must be tipo PATRIMONIO");
    assert_eq!(resultados["naturaleza"], "ACREEDORA", "3200 must be naturaleza ACREEDORA");
}
