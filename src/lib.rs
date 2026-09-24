use std::{char, collections::VecDeque, env, fmt::{Display, Error}, ops::AddAssign};

use anyhow::{Context, Result, bail, ensure};



pub fn process(filecontent: &String) -> Result<String> {
	let lines = filecontent.lines().collect::<Vec<&str>>();
	let meta = Metadata::new(lines.as_slice())?;
	dbg!(&meta.0);
	return Ok(meta.1);
}



#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Metadata<'metadata_fields> {
	lang: &'metadata_fields str,
	canonical: &'metadata_fields str,
	title: &'metadata_fields str,
	header: &'metadata_fields str,
	description: &'metadata_fields str,
}


impl<'output> Metadata<'output> {
	fn new<'inputs>(lines: &[&'inputs str]) -> Result<(Self, String)>
		where 'inputs: 'output,
	{
		if let [doctype, header, content @ .., closing_tag] = lines {
			ensure!(doctype.starts_with("<!DOCTYPE ghtml-v1.0 \"") && doctype.ends_with("\">"), "Invalid G-HTML structure: Invalid doctype! Expected the 1st line to start with „<!DOCTYPE ghtml-v1.0 \"” and end with „\">”, but got „{}” instead.", doctype);
			ensure!(closing_tag.to_string() == "</html>", "Invalid G-HTML structure: No valid closing tag! Expected the last line to be „</html>”, but got „{}” instead.", closing_tag);
			
			let (lang, next_header_segment) = header.strip_prefix("<html flavor=\"ghtml\" lang=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to start with „<html flavor=\"ghtml\" lang=\"”, but got „{}” instead.", header))?
				.split_once("\" canonical=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" canonical=\"” after the the lang param, but got „{}” instead.", header))?;
			let (canonical, next_header_segment) = next_header_segment.split_once("\" title=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" title=\"” after the the canonical param, but got „{}” instead.", header))?;
			let (title, next_header_segment) = next_header_segment.split_once("\" header=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" header=\"” after the the title param, but got „{}” instead.", header))?;
			let (header, next_header_segment) = next_header_segment.split_once("\" description=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" description=\"” after the the header param, but got „{}” instead.", header))?;
			let description = next_header_segment.strip_suffix("\">").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to end with a „\">” after the the description param, but got „.....{}” instead.", next_header_segment))?;
	
			return Ok((Metadata{lang, canonical, title, header, description}, content.join("\n")));
		} else {
			bail!("Not enough lines provided! Got {}, but expected at least 4.", lines.len());
		}
	}
}



#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Walker {
	//Walker state
	index: usize,
	complete: Complete,
	on: String,

	//Input line state
	indent: Renderable,
	indent_completion: Complete,
	active_tags: Vec<ParameterizedHtmlTag>,
	
	//Collected data (Line = output line!)
	past_lines: VecDeque<Renderable>,
	active_line: Renderable,
	word: VecDeque<WordSection>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Renderable {
	length: usize,
	content: String,
}

type Complete = bool;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WordSection {
	Literal(String),
	HtmlTag(HtmlTag, Complete),
	VarReplacement(String, Complete),
	HtmlEntity(String, Complete),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum HtmlTag {
	#[default] JustStarted,
	Opening(ParameterizedHtmlTag),
	Closing(String),
	SelfClosing(ParameterizedHtmlTag),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParameterizedHtmlTag {
	tag: String,
	args: Option<String>,
}


impl AddAssign for Renderable{
	fn add_assign(&mut self, rhs: Self) {
		self.length+=rhs.length;
		self.content.push_str(rhs.content.as_str());
	}
}

impl Display for Renderable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return self.content.fmt(f);
	}
}

impl Display for WordSection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self.render() {
			Ok(result) => result.fmt(f),
			Err(_) => Err(Error)
		}
	}
}

impl Display for HtmlTag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self.render() {
			Ok(result) => result.fmt(f),
			Err(_) => Err(Error)
		}
	}
}


impl HtmlTag {
	fn render(&self) -> Result<Renderable> {
		let content = match self {
			HtmlTag::JustStarted => bail!("Cannot render an incomplete tag!"),
			HtmlTag::Closing(tag) => format!("</{}>", tag),
			HtmlTag::Opening(tag) | HtmlTag::SelfClosing(tag) => {
				let ending = match self {
					HtmlTag::SelfClosing(_) => " />",
					_ => ">"
				};
				match &tag.args {
					Some(args) => format!("<{} {}{}", tag.tag, args, ending),
					None => format!("<{}{}", tag.tag, ending),
				}
			}
		};

		//TODO: Do envar replacement in tag params

		let length: usize = 0; //TODO: Add a tag param that overrides the size (for eg. emoji-style images, that should be rendered in-line as if they were letters (size 1 instead of 0) or for dummy tags that simply do nothing except for adding -1 to account for stuff like ligatures)

		return Ok(Renderable { length, content });
	}
}

impl WordSection {
	fn render(&self) -> Result<Renderable> {
		return match self {
			WordSection::Literal(literal) => Ok(Renderable{length: literal.chars().count(), content: literal.to_string().replace(" ", "&nbsp;")}),
			WordSection::HtmlTag(tag, true) => tag.render(),
			WordSection::HtmlEntity(entity, true) => Ok(Renderable{length: 1, content: format!("&{};", entity)}),

			WordSection::VarReplacement(varname, true) => match env::var(varname) {
				Ok(content) => Ok(Renderable{length: content.chars().count(), content}),
				Err(env::VarError::NotPresent) =>  bail!("Attempted to render an envar %{}% that doesn't exist!", varname),
				Err(env::VarError::NotUnicode(os_string)) => {
					let rs_string = os_string.to_string_lossy();
					return Ok(Renderable{length: rs_string.chars().count(), content: rs_string.to_string().replace(" ", "&nbsp;")});
				}
			}

			WordSection::HtmlTag(_, false) | WordSection::VarReplacement(_, false) | WordSection::HtmlEntity(_, false) => bail!("Attempted to render an incomplete WordSection!")
		}
	}

	fn from_char(previous: Option<&Self>, chr: char) -> Result<Option<Self>> {
		let current = match chr {
			'\n' => bail!("Cannot construct a new word section when switching lines!"),
			' ' => None,
			'<' => Some(WordSection::HtmlTag(HtmlTag::JustStarted, false)),
			'%' => Some(WordSection::VarReplacement("".to_string(), false)),
			'&' => Some(WordSection::HtmlEntity("".to_string(), false)),
			_ => Some(WordSection::Literal(chr.to_string()))
		};

		return match previous {
			Some(WordSection::Literal(base)) => match current {
				Some(WordSection::Literal(continuation)) => {
					let mut base_mut = base.to_string();
					base_mut.push_str(continuation.as_str());
					return Ok(Some(WordSection::Literal(base_mut)));
				}
				_ => Ok(current),
			}
			Some(WordSection::HtmlTag(_, false)) => match current {
				Some(WordSection::HtmlTag(_, _)) => todo!("Implement support for continuing HTML tags."),
				Some(WordSection::VarReplacement(_, _)) => todo!("Implement support for continuing HTML tags."),
				Some(WordSection::HtmlEntity(_, _)) => todo!("Implement support for continuing HTML tags."),
				Some(WordSection::Literal(_)) => todo!("Implement support for continuing HTML tags."),
				None => todo!("Implement support for continuing HTML tags."),
			}
			Some(WordSection::VarReplacement(base, false)) => match current {
				Some(WordSection::Literal(continuation)) => {
					let mut base = base.to_string();
					base.push_str(continuation.as_str());
					return Ok(Some(WordSection::VarReplacement(base, false)));
				}
				Some(WordSection::VarReplacement(_, _)) => Ok(Some(WordSection::VarReplacement(base.to_string(), true))),
				_ => bail!("Cannot use {} inside a var-replacement!", chr),
			}
			Some(WordSection::HtmlEntity(_, false)) => match current {
				Some(WordSection::HtmlTag(_, _)) => todo!("Implement support for continuing HTML entities."),
				Some(WordSection::VarReplacement(_, _)) => todo!("Implement support for continuing HTML entities."),
				Some(WordSection::HtmlEntity(_, _)) => todo!("Implement support for continuing HTML entities."),
				Some(WordSection::Literal(_)) => todo!("Implement support for continuing HTML entities."),
				None => todo!("Implement support for continuing HTML entities."),
			}
			Some(WordSection::HtmlEntity(_, true) | WordSection::VarReplacement(_, true) | WordSection::HtmlTag(_, true)) => bail!("Cannot build off of a previous segment, if that segment is already completed! There's simply nothing that could possibly be added."),
			None => match current {
				Some(section) => match section {
					WordSection::Literal(_) => match chr {
						'>' => Ok(Some(WordSection::HtmlEntity("gt".to_string(), true))),
						_ => Ok(Some(section))
					}
					_ => Ok(Some(section)),
				}
				None => Ok(Some(WordSection::HtmlEntity("nbsp".to_string(), true))),
			},
		}
	}
}

impl Walker {
	pub fn walk(mut self) -> Result<Self> {
		//Known special chars
		let indent_chars = ['|', ' ', '\\', '*', '-', '[', '/'];

		//Setup
		ensure!(!self.complete, "Tried to continue walking even after the walker reached the end!");
		let indexable_on = self.on.chars().collect::<Vec<char>>();
		let current = indexable_on.get(self.index).with_context(|| format!("Tried to index at position {} (+1 because 0-indexed arrays) for a string „{}” that only has {} characters! Note, that this error normally should never happen because an earlier \"Is complete?\" check should've caught it. Something must've gone seriously wrong (Were \"on:\", \"complete\" or \"index\" unsafely messed with? Was a zero-length string passed to work on?).", self.index, self.on, indexable_on.len()))?;

		//Main logic
		if self.active_line.length == 0 && self.word.is_empty(){
			dbg!(format!("At char „{}” (#{} in „{})”, we're at a beginning of a new line.", current, self.index, self.on));

			//State sanity-check and reset
			ensure!(self.active_tags.is_empty(), "Tried to start a new line (at char {}, #{} in „{}”), but some tags remained unclosed on the previous line!.", current, self.index, self.on);
			self.indent_completion = false;
			self.indent.content = "".to_string();
			self.indent.length = 0;

			//Line init strategies
			if *current == '\n' {
				dbg!(format!("It seems to be empty!"));
				self.past_lines.push_back(Renderable { length: 0, content: "".to_string() });
			}
			else if indent_chars.contains(current) {
				dbg!(format!("New line begins with an indent in form of a {}.", current));
				self.append_indent_char(*current).with_context(|| format!("Indent append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?;
			}
			else {
				self.word.push_back(WordSection::from_char(None, *current).with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?.with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”: Got an unescaped space character (represented by a None variant), which should be impossible at the beginning of a line because spaces at word beginnings (which includes line beginnings) should be auto-escaped, and also no space should even make it that far down anyway because it should've instead been consumed by the indent-appending code.", current, self.index, self.on))?);
				self.indent_completion = true;
			}
		} else {
			dbg!(format!("At char „{}” (#{} in „{})”, we're continuing a line.", current, self.index, self.on));
			if !self.indent_completion && indent_chars.contains(current) {
				dbg!(format!("Which means we're continuing an indent, in form of a {}.", current));
				self.append_indent_char(*current).with_context(|| format!("Indent append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?;
			}
			else if *current == '\n' {
				dbg!(format!("...Nevermind, we're ending it."));
				let ctx = format!("Tried to complete a line after char „{}” (#{} in „{}”), but it failed:", current, self.index, self.on);
				self = self.end_word().with_context(||ctx)?;
				self.past_lines.push_back(self.active_line);
				self.active_line = Renderable { length: 0, ..Renderable::default() } //A new value must be assigned because the previous one was moved out of self's ownership. (and also we need to ensure that length=0 so that the „we're at the beginning of a new line” logic runs on the next pass)
			}
			else {
				let mut previous = self.word.pop_back();
				let mut spaces_should_be_breaks_regardless_of_what_wordsection_fromchar_returns = false;
				//If the previous section is holding something that WordSection::from_char cannot comprehend (ie. a finished section), we put it back into the word, and instead give WordSection::from_char a fresh section to work with.
				match previous {
					Some(WordSection::HtmlEntity(_, true) | WordSection::VarReplacement(_, true) | WordSection::HtmlTag(_, true)) => {
						// SAFETY: This code is only reachable if we just match{}ed that "previous" is holding a Some().
						self.word.push_back(unsafe{previous.unwrap_unchecked()});
						previous = None;
						spaces_should_be_breaks_regardless_of_what_wordsection_fromchar_returns = true; //This override of default behaviors is needed because when WordSection::from_char sees a space at what-it-thinks-is-the-beginning-of-a-word (and we just gaslit it into thinking that it, indeed, is at the beginning), it automatically escapes it to be an NBSP. We don't want that here; just because a word ended on a tag/entity/envar section, doesn't mean we should forcefully merge it with the next word.
					},
					_ => ()
				};

				let next = if spaces_should_be_breaks_regardless_of_what_wordsection_fromchar_returns && *current == ' ' { None } else {
					WordSection::from_char(previous.as_ref(), *current).with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?
				};

				match next {
					None => {
						dbg!(format!("At char „{}” (#{} in „{})”, we're completing a word.", current, self.index, self.on));
						let ctx = format!("Tried to complete a word after char „{}” (#{} in „{}”), but it failed:", current, self.index, self.on);
						self = self.end_word().with_context(||ctx)?;
					},
					Some(section) => {
						self.word.push_back(section);
					}
				};
			}
		}

		//Increment and exit
		self.index+=1;
		if self.index == indexable_on.len() {
			let ctx = format!("Tried to complete the walk after char „{}” (#{} in „{}”), but the necessary end_word subtask failed:", current, self.index-1, self.on);
			self = self.end_word().with_context(||ctx)?;
			ensure!(self.active_tags.is_empty(), "Tried to complete the walk after char „{}” (#{} in „{}”), but some tags remained unclosed on the previous line!", current, self.index-1, self.on);
			self.past_lines.push_back(self.active_line);
			self.active_line = Renderable::default();
			self.complete = true;
		}
		return Ok(self);
	}

	fn append_indent_char(&mut self, chr: char) -> Result<()> {
		ensure!(!self.indent_completion, "Tried to append an indent char „{}” to a line that already completed its indent!", chr);

		let chr_str = chr.to_string();
		let nbsp = "&nbsp;";

		self.active_line.content.push_str(match chr {
			' ' => nbsp,
			_ => chr_str.as_str(),
		});
		self.indent.content.push_str(match chr {
			'|' => "|",
			_ => nbsp,
		});

		self.active_line.length+=1;
		self.indent.length+=1;

		return Ok(());
	}

	fn end_word(mut self) -> Result<Self> {
		const LENGTH_LIMIT: usize = 54;

		//STEP 1: Render the sections.
		let mut rendered = Renderable::default();
		let mut future_active_tags = self.active_tags.clone(); //Cloning needed because we want to be able to roll-back to a backup copy (the og active_tags) when needed; that's the whole point of this var existing (if I didn't need a backup, I'd've worked on active_tags directly and AFAIK the borrow checker wouldn't complain).
		loop {
			let section = match self.word.pop_front() {
				Some(section) => section,
				None => break,
			};
			let rendered_section = section.render().with_context(|| format!("Section rendering failed at char #{} in „{}”:", self.index, self.on))?;
			if let WordSection::HtmlTag(tag, true) = section {
				match tag {
					HtmlTag::Opening(tag) => future_active_tags.push(tag),
					HtmlTag::Closing(tag) => {
						let closes = future_active_tags.pop().with_context(|| format!("Tried to close </{}> at char #{} in „{}”, but there wasn't anything to close!", tag, self.index, self.on))?.tag;
						ensure!(tag == closes, "Tried to close </{}> at char #{} in „{}”, but </{}> was expected instead!", tag, self.index, self.on, closes);
					}
					_ => (),
				}
			}
			rendered += rendered_section;
		}

		//STEP 2A: Combine with whatever's already on the line (easy case)
		if !(rendered.length + self.active_line.length > LENGTH_LIMIT) {
			self.active_line += rendered;
			self.active_tags = future_active_tags;
			return Ok(self);
		}

		//STEP 2B: line-wrapping (painful) - STEP 3: Terminate previous line
		let mut tags_to_reopen = Vec::default();
		loop {
			let tag = match self.active_tags.pop() {
				Some(tag) => tag,
				None => break,
			};
			self.active_line += HtmlTag::Closing(tags_to_reopen.push_mut(tag).tag.to_string()).render().with_context(|| format!("HTML tag rendering failed at char #{} in „{}”:", self.index, self.on))?;
		}
		self.past_lines.push_back(self.active_line);
		self.active_tags = future_active_tags;

		//STEP 4: Start new line.
		self.active_line = self.indent.clone(); //a "makes the borrow-checker happy"-noob-clone
		loop {
			let tag = match tags_to_reopen.pop() {
				Some(tag) => tag,
				None => break,
			};
			self.active_line += HtmlTag::Opening(tag).render().with_context(|| format!("HTML tag rendering failed at char #{} in „{}”:", self.index, self.on))?;
		}

		//STEP 5: Combine with whatever's on the just-created line, and make sure that it still fits.
		self.active_line += rendered;
		if self.active_line.length > LENGTH_LIMIT {
			bail!("Even after being moved to a new line, „{}” was just too gosh-damn DummyThiccc™ to fit in the {}-char limit (reached {} chars after combining with the indent and re-opened tags)", self.active_line, LENGTH_LIMIT, self.active_line.length);
		} else {
			return Ok(self);
		}
	}

	pub fn new(target: String) -> Self {
		return Walker { on: target, ..Self::default() };
	}
}