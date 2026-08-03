#![cfg(test)]

use crate::{read_varint, write_varint};

#[test]
fn three_is_three(){
    assert!(3 == 3)
}

#[test]
fn varints() {
    let buf = [0x3f, 12, 0x41, 255, 0x80, 0, 255, 255, 0xe0, 0, 0, 0, 0, 0, 0, 1];
    let mut index = 0;
    
    assert_eq!(read_varint(&buf, &mut index), 63);
    assert_eq!(read_varint(&buf, &mut index), 12);
    assert_eq!(read_varint(&buf, &mut index), 511);
    assert_eq!(read_varint(&buf, &mut index), 65535);
    assert_eq!(read_varint(&buf, &mut index), 2305843009213693953);

    assert_eq!(index, 16);

    let mut buf2 = [0; 16];
    let mut index2 = 0;

    write_varint(&mut buf2, &mut index2, 63);
    write_varint(&mut buf2, &mut index2, 12);
    write_varint(&mut buf2, &mut index2, 511);
    write_varint(&mut buf2, &mut index2, 65535);
    write_varint(&mut buf2, &mut index2, 2305843009213693953);

    assert_eq!(index2, 16);
    assert_eq!(buf, buf2);
}