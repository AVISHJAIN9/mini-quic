# Systems Engineering Glossary: Mini-QUIC Protocol Server

### QUIC
Multiplexed transport protocol built on top of UDP providing connection migration, zero-RTT connection resumption, and encryption.

### Head-of-Line Blocking
Delay where packet loss on one stream stalls delivery of other independent streams; eliminated by QUIC.

### Connection ID
Cryptographic identifier enabling transport connections to survive client IP address and port network migrations.

### Stream Multiplexing
Transmitting multiple independent byte streams simultaneously over a single underlying UDP socket.
