use std::{collections::HashMap};

pub struct VM {
    stack:   Vec<[u8; 32]>,
    pc:      u64,
    gas:     u64,
    memory:  Vec<u8>,
    storage: HashMap<[u8; 32], [u8; 32]>,
}

impl VM {

    //---- HELPER TO CHECK IF MEM IS ENOUGH-------

    pub fn ensure_memory(&mut self, offset : usize , size : usize) {
        if self.memory.len() < offset + size{ 
            self.memory.resize(offset+size, 0u8);
        }
    }

    pub fn new() -> Self {
        VM {
            stack:   Vec::new(),
            pc:      0,
            gas:     0,
            memory:  Vec::new(),
            storage: HashMap::new(),
        }
    }

    pub fn execute(&mut self, bytecode: &[u8]) -> Result<(), String> {
        while (self.pc as usize) < bytecode.len() {
            let opcode = bytecode[self.pc as usize];
            self.pc += 1;

            match opcode {
                // STOP
                0x00 => return Ok(()),

                // ADD: pop two, push sum (wrapping 256-bit)
                0x01 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(add_u256(a, b));
                }

                // MUL (same as add but uses mul_256)
                0x02 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(mul_u256(a ,b));
                }

                0x03 =>  {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(sub_u256(a,b));
                }
                // DIV
                0x04 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(div_u256(a,b));
                }

                // LT: a < b
                0x10 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(bool_to_u256(cmp_u256(&a, &b) == std::cmp::Ordering::Less));
                }

                // GT: a > b
                0x11 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(bool_to_u256(cmp_u256(&a, &b) == std::cmp::Ordering::Greater));
                }

                // EQ: a == b
                0x14 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(bool_to_u256(a == b));
                }

                // ISZERO: a == 0
                0x15 => {
                    let a = self.pop()?;
                    self.push(bool_to_u256(a == [0u8; 32]));
                }

                // PUSH1: read next byte, push it as a 32-byte value
                0x60 => {
                    if (self.pc as usize) >= bytecode.len() {
                        return Err("PUSH1: missing operand".into());
                    }
                    let byte = bytecode[self.pc as usize];
                    self.pc += 1;
                    let mut val = [0u8; 32];
                    val[31] = byte;
                    self.push(val);
                }
                0x16 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(and_u256(a, b));
                }

                0x17 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(or_u256(a, b));
                }

                0x18 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(xor_u256(a, b));
                }

                0x19 => { 
                    let a = self.pop()?;
                    self.push(not_u256(a));
                }

                // ----- EVM MEMORY SHINANIGANS -----

                //1.MSTORE

                0x52 => { 
                    let offset = self.pop()?;

                    let offset = u64::from_be_bytes(
                        offset[24..32].try_into().unwrap()
                    ) as usize;
                    
                    let val = self.pop()?; 
                    self.ensure_memory(offset,32);

                    self.memory[offset..offset+32].copy_from_slice(&val);
                }

                //2.MLOAD

                0x51 => { 

                    let offset = self.pop()?;
                    let offset = u64::from_be_bytes(
                        offset[24..32].try_into().unwrap()
                    ) as usize;

                    self.ensure_memory(offset, 32);
                 
                    let mut result = [0u8; 32]; 

                    result.copy_from_slice(&self.memory[offset..offset+32]);

                    self.push(result);
                }

                // + MSSTORE8 (store last 8 bits of a val)
                // + see how its just self.memory[offset] = val[31]
                // + just yank the last 8 bits 

                0x53 => { 
 
                    let offset = self.pop()?;
                    let offset = u64::from_be_bytes(
                        offset[24..32].try_into().unwrap()
                    ) as usize;

                    let val = self.pop()?; 
                    
                    self.ensure_memory(offset,1);
                    self.memory[offset] = val[31]
                }
                
                //-------STORAGEE--------

                //1.SLOAD
                
                0x54 => { 
                    let key = self.pop()?; 
                    self.push(self.storage.get(&key).copied().unwrap_or([0u8;32]));
                }

                //2.SSTORE
                0x55 => { 
                    let key = self.pop()?; 
                    let val = self.pop()?; 
                    self.storage.insert(key, val);
                }

                //JUMP 
                // JUMP TO A VALID DESTINATION 
                // CHECK IF DESTINATION IS VALID.
                // RETURNS INVALID DEST ERR IF NOT.

                0x56 => { 
                    let dest = self.pop()?; 

                    let dest = u64::from_be_bytes(dest[24..32].try_into().unwrap());
                    
                    if bytecode[dest as usize] == 0x5b { 
                        self.pc = dest as u64;
                    }else {
                        return Err("JUMP: invalid destinaton".into());
                    }
                }

                // JUMPI
                // only jumps if conditon is not equals to zero 
                // otherwise its the same as JUMP
                0x57 => { 
                    let dest = self.pop()?; 
                    let dest = u64::from_be_bytes(dest[24..32].try_into().unwrap());

                    let cond = self.pop()?;

                    if cond != [0u8; 32] { 
                        if bytecode[dest as usize] == 0x5b { 
                            self.pc = dest as u64;

                        } else {
                            return Err("JUMP: invalid destinaton".into());
                        }
                    }
                }

                0x5b => {}
            
                // POP: discard top of stack
                0x50 => { self.pop()?; }

                // DUP1-DUP16
                0x80..=0x8f => {
                    let n = (opcode - 0x80 + 1) as usize;
                    let len = self.stack.len();
                    if len < n {
                        return Err(format!("DUP{}: stack underflow", n));
                    }
                    let val = self.stack[len - n];
                    self.push(val);
                }

                // SWAP1-SWAP16
                0x90..=0x9f => {
                    let n = (opcode - 0x90 + 1) as usize;
                    let len = self.stack.len();
                    if len <= n {
                        return Err(format!("SWAP{}: stack underflow", n));
                    }
                    self.stack.swap(len - 1, len - 1 - n);
                }

                unknown => return Err(format!("unknown opcode: 0x{:02x}", unknown)),
            }
        }

        Ok(())
    }

    fn push(&mut self, val: [u8; 32]) {
        self.stack.push(val);
    }

    fn pop(&mut self) -> Result<[u8; 32], String> {
        self.stack.pop().ok_or_else(|| "stack underflow".into())
    }
}

// wrapping big-endian addition for 256-bit values
fn add_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut carry: u16 = 0;
    for i in (0..32).rev() {
        let sum = a[i] as u16 + b[i] as u16 + carry;
        result[i] = sum as u8;
        carry = sum >> 8;
    }
    result
}

fn sub_u256(a: [u8; 32], b :[u8; 32]) -> [u8; 32] { 
    let mut result = [0u8; 32]; 

    let mut borrow : i16 = 0 ; 

    for i in (0..32).rev() { 
        let diff = a[i] as i16 - b[i] as i16 - borrow;
        result[i] = diff as u8;

        borrow = diff >>  8;
    }

    result
}

fn div_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    if b == [0u8; 32] {
        return [0u8; 32];
    }

    let to_limbs = |x: [u8; 32]| -> [u64; 8] {
        let mut limbs = [0u64; 8];
        for i in 0..8 {
            limbs[i] = u32::from_be_bytes(x[i*4..(i+1)*4].try_into().unwrap()) as u64;
        }
        limbs
    };

    let from_limbs = |limbs: [u64; 8]| -> [u8; 32] {
        let mut result = [0u8; 32];
        for i in 0..8 {
            result[i*4..(i+1)*4].copy_from_slice(&(limbs[i] as u32).to_be_bytes());
        }
        result
    };

    let cmp_limbs = |x: &[u64; 8], y: &[u64; 8]| -> std::cmp::Ordering {
        for i in 0..8 { if x[i] != y[i] { return x[i].cmp(&y[i]); } }
        std::cmp::Ordering::Equal
    };

    let shl1 = |x: &[u64; 8]| -> [u64; 8] {
        let mut r = [0u64; 8];
        let mut carry = 0u64;
        for i in (0..8).rev() {
            r[i] = ((x[i] << 1) | carry) & 0xFFFF_FFFF;
            carry = x[i] >> 31;
        }
        r
    };

    let sub_limbs = |x: &[u64; 8], y: &[u64; 8]| -> [u64; 8] {
        let mut r = [0u64; 8];
        let mut borrow = 0i64;
        for i in (0..8).rev() {
            let d = x[i] as i64 - y[i] as i64 - borrow;
            r[i] = (d & 0xFFFF_FFFF) as u64;
            borrow = if d < 0 { 1 } else { 0 };
        }
        r
    };

    let mut quotient = [0u64; 8];
    let mut remainder = [0u64; 8];
    let a_limbs = to_limbs(a);

    for bit in (0..256).rev() {
        remainder = shl1(&remainder);
        remainder[7] |= (a_limbs[7 - bit / 32] >> (bit % 32)) & 1;
        let b_limbs = to_limbs(b);
        if cmp_limbs(&remainder, &b_limbs) != std::cmp::Ordering::Less {
            remainder = sub_limbs(&remainder, &b_limbs);
            quotient[7 - bit / 32] |= 1 << (bit % 32);
        }
    }

    from_limbs(quotient)
}

fn cmp_u256(a: &[u8; 32], b: &[u8; 32]) -> std::cmp::Ordering {
    a.cmp(b)
}

fn bool_to_u256(v: bool) -> [u8; 32] {
    let mut r = [0u8; 32];
    r[31] = v as u8;
    r
}

fn mul_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let a_hi = u128::from_be_bytes(a[0..16].try_into().unwrap());
    let a_lo = u128::from_be_bytes(a[16..32].try_into().unwrap());
    let b_hi = u128::from_be_bytes(b[0..16].try_into().unwrap());
    let b_lo = u128::from_be_bytes(b[16..32].try_into().unwrap());

    let (a0, a1) = (a_lo >> 64, a_lo & u64::MAX as u128);
    let (b0, b1) = (b_lo >> 64, b_lo & u64::MAX as u128);
    let p0 = a1 * b1;
    let p1 = a1 * b0;
    let p2 = a0 * b1;
    let p3 = a0 * b0;
    let mid = (p0 >> 64) + (p1 & u64::MAX as u128) + (p2 & u64::MAX as u128);
    let ll_lo = (mid << 64) | (p0 & u64::MAX as u128);
    let ll_hi = p3 + (p1 >> 64) + (p2 >> 64) + (mid >> 64);

    let result_hi = ll_hi
        .wrapping_add(a_hi.wrapping_mul(b_lo))
        .wrapping_add(a_lo.wrapping_mul(b_hi));

    let mut result = [0u8; 32];
    result[0..16].copy_from_slice(&result_hi.to_be_bytes());
    result[16..32].copy_from_slice(&ll_lo.to_be_bytes());
    result
}

//-------------- BITWISE OPS --------

// AND op
fn and_u256(a: [u8;32 ], b:[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..32 { 
        result[i]  = a[i] & b[i]
    }
    result
}

// OR op
fn or_u256(a: [u8;32 ], b:[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..32 { 
        result[i]  = a[i] |  b[i]
    }
    result
}


// XOR op

fn xor_u256(a: [u8;32 ], b:[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..32 { 
        result[i]  = a[i] ^ b[i]
    }
    result
}

// NOT op

fn not_u256(a: [u8;32 ]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..32 { 
        result[i]  = !a[i]
    }
    result
}

