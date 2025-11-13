package util

type TemplateData struct {
	Name    string
	Content *string
}

type LineData struct {
	TypedLength      int
	BytesLength      int
	ProcessedContent string
}

type DocumentData struct {
	Lang        string
	Canonical   string
	Title       string
	Header      string
	Description string
}