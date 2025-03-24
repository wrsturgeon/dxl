use {nix::{net::if_::InterfaceFlags, sys::socket::{SockaddrStorage, SockaddrLike as _, AddressFamily}, ifaddrs::{InterfaceAddress, getifaddrs}}, std::net::{Ipv4Addr, UdpSocket}};

/*
fn getaddr() -> SockaddrStorage {
    let addrs = getifaddrs().unwrap();
    'addrs: for addr in addrs {
        let Some(ref address) = addr.address else { continue 'addrs };
        if !matches!(address.family(), Some(AddressFamily::Inet)) { continue 'addrs }
        if addr.flags.contains(InterfaceFlags::IFF_LOOPBACK) { continue 'addrs }
        if !addr.flags.contains(InterfaceFlags::IFF_UP & InterfaceFlags::IFF_RUNNING) { continue 'addrs }
        println!("Found a running IPv4 address: {addr:#?}");
        return addr.address.unwrap()
    }
    panic!("No running IPv4 address found");
}
*/

/*
fn make_a_socket_god_damn_it() -> UdpSocket {
    for a in u8::MIN..=u8::MAX {
        for b in u8::MIN..=u8::MAX {
            if let Ok(ok) = UdpSocket::bind((Ipv4Addr::new(192, 168, a, b), 50_000)) {
                println!("{ok:#?}");
                return ok;
            }
        }
    }
    panic!("literally nothing worked");
}
*/

#[derive(Debug)]
struct Positions {
    p21: u16,
    p22: u16,
    p23: u16,
    p24: u16,
    p25: u16,
    p26: u16,
    p41: u16,
    p42: u16,
    p43: u16,
}

fn main() {

    // let socket = make_a_socket_god_damn_it();
    // let socket = UdpSocket::bind((Ipv4Addr::new(192, 168, 4, 2), 5_000)).unwrap();
    let socket = UdpSocket::bind((Ipv4Addr::new(0, 0, 0, 0), 5_000)).unwrap();

    // let n_bytes = socket.send_to(&[0; 10], "169.254.1.1:1234").unwrap();
    // println!("Sent {n_bytes} bytes");

    let mut positions = Positions {
        p21: 32768,
        p22: 32768,
        p23: 32768,
        p24: 32768,
        p25: 32768,
        p26: 32768,
        p41: 32768,
        p42: 32768,
        p43: 32768,
    };
    let mut buffer = [0; 256];

    loop {
        let (n_bytes, endpoint) = socket.recv_from(&mut buffer).unwrap();
        // println!("Received {:?} ({} bytes) from {:?}", core::str::from_utf8(&buffer[..n_bytes]), n_bytes, endpoint);
        
        if n_bytes != 10 {
            continue
        }

        let to_edit: &mut u16 = match &buffer[..5] {
            b"/021/" => &mut positions.p21,
            b"/022/" => &mut positions.p22,
            b"/023/" => &mut positions.p23,
            b"/024/" => &mut positions.p24,
            b"/025/" => &mut positions.p25,
            b"/026/" => &mut positions.p26,
            b"/041/" => &mut positions.p41,
            b"/042/" => &mut positions.p42,
            b"/043/" => &mut positions.p43,
            other => panic!("unrecognized ID: {:?}", core::str::from_utf8(other)),
        };

        let parsed_int = {
            let mut i = (buffer[5] - b'0') as u16;
            i = 10 * i + (buffer[6] - b'0') as u16;
            i = 10 * i + (buffer[7] - b'0') as u16;
            i = 10 * i + (buffer[8] - b'0') as u16;
            i = 10 * i + (buffer[9] - b'0') as u16;
            i
        };

        *to_edit = parsed_int;

        println!("{positions:04X?}");
    }
}
