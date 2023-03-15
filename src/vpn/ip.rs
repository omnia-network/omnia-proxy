use std::net::Ipv4Addr;

type Ipv4Mask = Ipv4Addr;

pub fn next_ipv4_address(ip: Ipv4Addr, netmask: Ipv4Mask) -> Option<Ipv4Addr> {
    // Convert the IPv4 address and netmask to 32-bit unsigned integers
    let ip_num = u32::from(ip);
    let netmask_num = u32::from(netmask);

    // Compute the network prefix by masking the IP address with the netmask
    let network_prefix = ip_num & netmask_num;

    // Compute the range of addresses within the network by setting all bits outside the netmask to 1
    let network_range = netmask_num ^ u32::MAX;

    // Increment the IP address within the network range
    let next_ip_num = network_prefix
        .checked_add(1)?
        .min(network_prefix + network_range);

    // Convert the result back to an IPv4 address
    Some(Ipv4Addr::from(next_ip_num))
}

pub fn next_available_ipv4_address(ip_addrs: &[Ipv4Addr], netmask: Ipv4Mask) -> Option<Ipv4Addr> {
    let netmask_num = u32::from(netmask);
    let network_range = netmask_num ^ u32::MAX;
    let mut max_ip_num = u32::MIN;

    for &ip in ip_addrs {
        let ip_num = u32::from(ip);
        // Check if the IP address is within the network range and is greater than the current maximum
        if (ip_num & netmask_num) == (ip_num & network_range) && ip_num > max_ip_num {
            max_ip_num = ip_num;
        }
    }

    if max_ip_num == u32::MIN {
        None
    } else {
        next_ipv4_address(Ipv4Addr::from(max_ip_num), netmask)
    }
}
