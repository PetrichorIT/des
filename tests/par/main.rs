use des::*;

fn main() {
    let mut par = Parameters::new();

    let raw = "
    netA.*.dnsServer = 1.1.1.1
    netA.s0.ip = 0.0.0.1

    netA.s1.ip = 0.0.0.1

    netA.s1.ipv6 = fe80
    ";

    par.build(raw);
}
