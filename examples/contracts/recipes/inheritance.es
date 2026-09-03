/**
 * You can spend at any time; a second person can spend only after a date.
 * If you never move the funds, they can claim them later.
 * @param owner Who owns the funds? — Your own address; can spend at any time.
 * @param heir Who inherits? — The address that may spend after the date.
 * @param heirHeight From when may the heir spend? — Before this only the owner can.
 */
@contract def inheritance(owner: SigmaProp, heir: SigmaProp, heirHeight: Int) =
  owner || (heir && sigmaProp(HEIGHT >= heirHeight))
