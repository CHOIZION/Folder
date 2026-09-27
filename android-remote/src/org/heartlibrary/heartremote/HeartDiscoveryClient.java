package org.heartlibrary.heartremote;

import android.content.Context;
import android.net.DhcpInfo;
import android.net.wifi.WifiManager;

import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashSet;
import java.util.Set;

// DhcpInfo is retained for reliable directed broadcasts on the app's API 26 baseline.
@SuppressWarnings("deprecation")
final class HeartDiscoveryClient {
    private static final int DISCOVERY_PORT = 37219;
    private static final int TIMEOUT_MS = 2600;
    private static final String REQUEST = "HEART_DISCOVER_V1";
    private static final String RESPONSE = "HEART_HOST_V1";

    private final Context appContext;

    HeartDiscoveryClient(Context context) {
        appContext = context.getApplicationContext();
    }

    FoundHost discover() throws Exception {
        try (DatagramSocket socket = new DatagramSocket()) {
            socket.setBroadcast(true);
            socket.setSoTimeout(TIMEOUT_MS);
            byte[] request = REQUEST.getBytes(StandardCharsets.UTF_8);
            for (InetAddress address : broadcastAddresses()) {
                socket.send(new DatagramPacket(request, request.length, address, DISCOVERY_PORT));
            }

            byte[] buffer = new byte[256];
            DatagramPacket response = new DatagramPacket(buffer, buffer.length);
            socket.receive(response);
            return parseResponse(response);
        }
    }

    private FoundHost parseResponse(DatagramPacket response) {
        String message = new String(
            response.getData(), 0, response.getLength(), StandardCharsets.UTF_8
        );
        String[] parts = message.split("\\|", 3);
        if (parts.length < 2 || !RESPONSE.equals(parts[0])) {
            throw new IllegalStateException("HEART 응답 형식이 올바르지 않습니다.");
        }
        int port = Integer.parseInt(parts[1]);
        String address = response.getAddress().getHostAddress();
        String name = parts.length >= 3 ? parts[2] : "HEART-PC";
        return new FoundHost(address, port, name);
    }

    private Set<InetAddress> broadcastAddresses() throws Exception {
        Set<InetAddress> addresses = new LinkedHashSet<>();
        addresses.add(InetAddress.getByName("255.255.255.255"));

        WifiManager wifi = (WifiManager) appContext.getSystemService(Context.WIFI_SERVICE);
        DhcpInfo dhcp = wifi == null ? null : wifi.getDhcpInfo();
        if (dhcp == null) return addresses;

        int broadcast = (dhcp.ipAddress & dhcp.netmask) | ~dhcp.netmask;
        byte[] quads = new byte[4];
        for (int index = 0; index < quads.length; index++) {
            quads[index] = (byte) ((broadcast >> (index * 8)) & 0xFF);
        }
        addresses.add(InetAddress.getByAddress(quads));
        return addresses;
    }

    static final class FoundHost {
        final String address;
        final int port;
        final String name;

        FoundHost(String address, int port, String name) {
            this.address = address;
            this.port = port;
            this.name = name;
        }

        String authority() {
            return address + ":" + port;
        }

        String baseUrl() {
            return "http://" + authority();
        }
    }
}
