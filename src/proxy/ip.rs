use std::net::Ipv4Addr;

use super::models::AssignedIpsMap;

type Ipv4Mask = Ipv4Addr;

fn next_ipv4_address(ip_num: u32, netmask: Ipv4Mask) -> Option<Ipv4Addr> {
    // Convert the IPv4 address and netmask to 32-bit unsigned integers
    let netmask_num = u32::from(netmask);

    // Compute the network prefix by masking the IP address with the netmask
    let network_prefix = ip_num & netmask_num;

    // Compute the range of addresses within the network by setting all bits outside the netmask to 1
    let network_range = netmask_num ^ u32::MAX;

    // Increment the IP address within the network range
    let next_ip_num = ip_num.checked_add(1)?.min(network_prefix + network_range);

    // Convert the result back to an IPv4 address
    Some(Ipv4Addr::from(next_ip_num))
}

pub fn next_available_ipv4_address(
    ip_addrs: &AssignedIpsMap,
    netmask: Ipv4Mask,
    first_addr: Ipv4Addr,
) -> Option<Ipv4Addr> {
    // set it to the first address in the network, which is the wireguard interface address
    let mut max_ip_num = u32::from(first_addr);

    for (&ip, _) in ip_addrs {
        let ip_num = u32::from(ip);
        // Check if the IP address is greater than the current maximum
        if ip_num > max_ip_num {
            max_ip_num = ip_num;
        }
    }

    next_ipv4_address(max_ip_num, netmask)
}
