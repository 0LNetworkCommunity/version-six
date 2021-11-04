//! account: alice, 1000000GAS, 0, validator
//! account: bob, 10GAS,
//! account: carol, 10GAS,


// META: transfers between bob and carol (not slow wallets) works fine.
// Note this test also exists standalone as _meta_pay_from. But keep a transaction here for comprehension.

//! new-transaction
//! sender: carol
script {
use 0x1::GAS::GAS;
use 0x1::DiemAccount;

fun main(account: signer) {
    assert(DiemAccount::balance<GAS>(@{{bob}}) == 10, 735701);

    let with_cap = DiemAccount::extract_withdraw_capability(&account);
    DiemAccount::pay_from<GAS>(&with_cap, @{{bob}}, 10, x"", x"");
    DiemAccount::restore_withdraw_capability(with_cap);
    assert(DiemAccount::balance<GAS>(@{{bob}}) == 20, 735701);
}
}

// check: EXECUTED

// This transaction should fail because alice is a slow wallet, and has no GAS unlocked.

//! new-transaction
//! sender: alice
script {
use 0x1::GAS::GAS;
use 0x1::DiemAccount;
fun main(account: signer) {
    assert(DiemAccount::unlocked_amount(@{{alice}}) == 0, 735701);

    let with_cap = DiemAccount::extract_withdraw_capability(&account);
    DiemAccount::pay_from<GAS>(&with_cap, @{{bob}}, 10, x"", x"");
    DiemAccount::restore_withdraw_capability(with_cap);
}
}


// check: ABORTED