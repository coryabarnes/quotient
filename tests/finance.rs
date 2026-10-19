//! Finance, checked against the examples in Microsoft's Excel documentation.

mod common;
use common::{close, error};

#[test]
fn payments() {
    close("pmt(8% / 12, 10, 10000)", -1037.0320893591636);
    close("pmt(8% / 12, 10, 10000, 0, 1)", -1030.1643271779772);
    close("pmt(6% / 12, 18 * 12, 0, 50000)", -129.0811608679954);
    close("pmt(0, 10, 1000)", -100.0);
    close("pmt(6.5% / 12, 360, 300000)", -1896.2040704789);
}

#[test]
fn values_and_periods() {
    close("fv(6% / 12, 10, -200, -500, 1)", 2581.4033740601362);
    close("fv(12% / 12, 12, -1000)", 12682.503013196976);
    close("fv(0, 10, -100)", 1000.0);
    close("pv(8% / 12, 12 * 20, 500)", -59777.14585118777);
    close("nper(12% / 12, -100, -1000, 10000, 1)", 59.67386567429457);
    close("nper(1%, -100, 1000)", 10.58864445942323);
    close("nper(1%, 5, 1000)", -110.40962404966885); // Excel gives the same negative count
    // Paying 50 a period never clears 1000 at 10% interest (Excel: #NUM!).
    assert!(error("nper(10%, -50, 1000)").contains("never"));
}

#[test]
fn cash_flows() {
    close("npv(10%, -10000, 3000, 4200, 6800)", 1188.4434123352216);
    close("irr(-70000, 12000, 15000, 18000, 21000, 26000)", 0.08663094803653149);
    close("irr([-70000, 12000, 15000, 18000, 21000])", -0.02124484827341105);
    close("irr(-100, 110)", 0.1);
    assert!(error("irr(100, 200)").contains("positive and one negative"));
}

#[test]
fn growth() {
    close("cagr(1000, 1500, 3)", 14.471424255333186);
    close("compound(1000, 5, 10)", 1000.0 * 1.05f64.powi(10));
    close("pctchange(80, 100)", 25.0);
}
