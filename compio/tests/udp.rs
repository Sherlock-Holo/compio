use compio::net::UdpSocket;

#[compio_macros::test]
async fn connect() {
    const MSG: &str = "foo bar baz";

    let mut passive = UdpSocket::bind("127.0.0.1:0").unwrap();
    let passive_addr = passive.local_addr().unwrap();

    let mut active = UdpSocket::bind("127.0.0.1:0").unwrap();
    let active_addr = active.local_addr().unwrap();

    active.connect(passive_addr).unwrap();
    active.send(MSG).await.0.unwrap();

    let (_, buffer) = passive.recv(Vec::with_capacity(20)).await.unwrap();
    assert_eq!(MSG.as_bytes(), &buffer);
    assert_eq!(active.local_addr().unwrap(), active_addr);
    assert_eq!(active.peer_addr().unwrap(), passive_addr);
}

#[compio_macros::test]
async fn send_to() {
    const MSG: &str = "foo bar baz";

    macro_rules! must_success {
        ($r:expr, $expect_addr:expr) => {
            let res = $r;
            assert_eq!(res.0.unwrap().1, $expect_addr);
            assert_eq!(res.1, MSG.as_bytes());
        };
    }

    let mut passive1 = UdpSocket::bind("127.0.0.1:0").unwrap();
    let passive1_addr = passive1.local_addr().unwrap();

    let passive01 = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let passive01_addr = passive01.local_addr().unwrap();

    let mut passive2 = UdpSocket::bind("127.0.0.1:0").unwrap();
    let passive2_addr = passive2.local_addr().unwrap();

    let mut passive3 = UdpSocket::bind("127.0.0.1:0").unwrap();
    let passive3_addr = passive3.local_addr().unwrap();

    let mut active = UdpSocket::bind("127.0.0.1:0").unwrap();
    let active_addr = active.local_addr().unwrap();

    active.send_to(MSG, &passive01_addr).await.0.unwrap();
    active.send_to(MSG, &passive1_addr).await.0.unwrap();
    active.send_to(MSG, &passive2_addr).await.0.unwrap();
    active.send_to(MSG, &passive3_addr).await.0.unwrap();

    must_success!(
        passive1.recv_from(Vec::with_capacity(20)).await,
        active_addr
    );
    must_success!(
        passive2.recv_from(Vec::with_capacity(20)).await,
        active_addr
    );
    must_success!(
        passive3.recv_from(Vec::with_capacity(20)).await,
        active_addr
    );
}
