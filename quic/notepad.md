
### quic listener
make custom trait for datagram sources to allow unusual sources <br/>
can directly pass Buffer `connection.handle_from(buffer)` </br>
single do all function `connection.next()` </br>
no internal buffer </br>

functions to decrypt then parse packets <br/>
will likely make everything use a buffer instead of parsing into struct <br/>
allow directly feeding decrypted packets <br/>
allow directly feeding frames <br/>

first actually implement it all, then rewrite to be performant <br/>


### pseudo code

```rust
pub struct QuicListener {
    /* ... */ 
}
pub struct QuicConnection { /* ... */ }
```
