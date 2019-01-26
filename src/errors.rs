error_chain!{
	foreign_links{
		IO(std::io::Error);
		PCSC(pcsc::Error);
	}
}
