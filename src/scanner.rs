#[derive(Clone)]
pub struct LineStart<'a> {
    bytes: &'a [u8],
    cur: usize,
    tab_start: usize,
    spaces_remaining: usize,
    min_hrule_offset: usize,
}

impl<'a> LineStart<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cur: 0,
            tab_start: 0,
            spaces_remaining: 0,
            min_hrule_offset: 0,
        }
    }

    pub fn has_next(&self) -> bool {
        self.cur < self.bytes.len()
    }

    pub fn peek(&self) -> Option<u8> {
        self.bytes.last().copied()
    }

    pub fn next(&mut self) -> u8 {
        let ch = self.bytes[self.cur];

        self.cur += 1;

        ch
    }

    pub fn is_at_eol(&self) -> bool {
        self.peek().map_or(true, |b| matches!(b, b'\r' | b'\n'))
    }

    pub fn skip_spaces(&mut self) {
        self.spaces_remaining = 0;
        self.cur += self.bytes[self.cur..]
            .iter()
            .take_while(|&&b| matches!(b, b' ' | b'\t'))
            .count();
    }

    pub fn scan_space(&mut self, mut n: usize) -> usize {
        let x = self.spaces_remaining.min(n);

        n -= x;
        self.spaces_remaining -= x;

        let n_save = 0;

        while n > 0 {
            match self.peek() {
                Some(b' ') => {
                    self.cur += 1;
                    n -= 1;
                }
                Some(b'\t') => {
                    let spaces = 4 - (self.cur - self.tab_start) % 4;
                    let x = n.min(spaces);

                    n -= x;
                    self.spaces_remaining = spaces - x;

                    self.cur += 1;
                    self.tab_start += self.cur;
                }
                _ => break,
            }
        }

        n_save - n
    }

    pub fn scan_ch(&mut self, ch: u8) -> bool {
        match self.peek() {
            Some(c) if c == ch => {
                self.cur += 1;
                true
            }
            _ => false,
        }
    }

    pub fn scan_blockquote_marker(&mut self) -> bool {
        self.try_scan(|this| {
            this.scan_space(3);
            if this.scan_ch(b'>') {
                this.scan_space(1);
                Ok(true)
            } else {
                Err(())
            }
        })
        .unwrap_or(false)
    }

    /// Scan a list marker.
    ///
    /// Return value is the character and the start index.
    /// For ordered list markers, the character will be one of b'.' or b')'. For
    /// bullet list markers, it will be one of b'-', b'+', or b'*'.
    pub fn scan_list_marker(&mut self) -> Option<(u8, u64)> {
        self.try_scan(|this| {
            this.scan_space(3);

            match this.peek() {
                Some(ch) if matches!(ch, b'-' | b'+' | b'*') => {
                    this.cur += 1;

                    if this.scan_space(1) == 1 || this.is_at_eol() {
                        Ok((ch, 0))
                    } else {
                        Err(())
                    }
                }
                Some(ch) if ch.is_ascii_digit() => {
                    let mut val = u64::from(ch - b'0');

                    this.cur += 1;

                    while let Some(ch) = this.peek() {
                        this.cur += 1;

                        if ch.is_ascii_digit() {
                            val = val * 10 + u64::from(ch - b'0');
                        } else if matches!(ch, b')' | b'.') {
                            if this.scan_space(1) == 1 || this.is_at_eol() {
                                return Ok((ch, val));
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    Err(())
                }
                _ => Err(()),
            }
        })
        .ok()
    }

    pub fn scan_task_list_marker(&mut self) -> Option<bool> {
        self.try_scan(|this| {
            this.scan_space(3);

            if !this.scan_ch(b'[') {
                return Err(());
            }

            let is_checked = match this.peek() {
                Some(b' ') => {
                    this.cur += 1;
                    false
                }
                Some(b'x') | Some(b'X') => {
                    this.cur += 1;
                    true
                }
                _ => return Err(()),
            };

            if !this.scan_ch(b']') {
                return Err(());
            }

            if this.scan_space(1) == 1 || this.is_at_eol() {
                Ok(is_checked)
            } else {
                Err(())
            }
        })
        .ok()
    }

    fn try_scan<F, R, E>(&mut self, mut scan: F) -> Result<R, E>
    where
        F: FnMut(&mut Self) -> Result<R, E>,
    {
        let clone = self.clone();

        match scan(self) {
            x @ Ok(..) => x,
            x @ Err(..) => {
                *self = clone;
                x
            }
        }
    }
}
