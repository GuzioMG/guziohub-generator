package cmd

import (
	"errors"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
)

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   " generate <flags> OR generate <content: File or Folder> <using: File> <to: Folder (unless \"content\" was a File) or Empty Path>",
	Short: "Builds my website",
	Long:  `Builds my website.`,
	Args:  cobra.ExactArgs(3),
	RunE: func(cmd *cobra.Command, args []string) error {
		//Basic setup
		multiContent := false
		toNeedsClosing := true
		if strings.HasPrefix(args[0], "."+string(os.PathSeparator)) || args[0] == "." {
			return errors.New("please use a different form of relative path (like some/subfolder not ./some/subfolder) and/or don't operate directly on the working directory (you CAN do ../[workdir] or use absolute paths for it) to avoid path-parsing issues") // I debugged this and know the cause of it (it's becasue filepath.Walk tries to be helpful and strips away any references to the workdir from its string path param - but that ends up interfering with my own prefix-stripping logic), but was too lazy to fix
		}

		// Load args
		content, err := os.Open(args[0])
		if err != nil {
			return err
		} else if desc, err := content.Stat(); err != nil {
			return err
		} else if desc.IsDir() {
			multiContent = true
		}
		defer content.Close()

		using, err := os.Open(args[1])
		if err != nil {
			return err
		} else if desc, err := using.Stat(); err != nil {
			return err
		} else if desc.IsDir() {
			return errors.New("the template file (\"using\" arg, aka arg #2) cannot be a directory")
		}
		defer using.Close()

		to, err := os.Open(args[2])
		if err != nil {
			if !os.IsNotExist(err) {
				return err
			}
			toNeedsClosing = false
		} else if desc, err := to.Stat(); err != nil {
			return err
		} else if (desc.IsDir() && !multiContent) || (!desc.IsDir() && multiContent) {
			return errors.New("FS type (File vs Folder) of the output (\"to\" arg, aka arg #3) must match that of the content (\"content\" arg, aka arg #1)")
		}
		defer to.Close() //This may technically return an error if 1) toNeedsClosing is false (ie. the file didn't even exist in the 1st place, so we have nothing to close) or 2) we closed it earlier-than-planned in the (see: "simple case" below). But we don't really care in either case because 1) if it fails to close because of a reason like that - great! it was already closed, so we got the desired end result, anyway 2) it's a defer statement, so it runs after the function already finished, so this won't cause us to return an err != nil and make Cobra display an error to the user, and 3) if we ended up closing it earlier-than-planned, then we have a bigger problem to worry about, anyway (it's about to be re-opened in os.O_EXCL mode, which cannot happen because the file already exists (if it didn't, toNeedsClosing would've gotten set to false, above) - so the user will get an error from that instead, which is more informative than an error from this Close() call would be).

		// Simple case
		if !multiContent {
			if toNeedsClosing {
				to.Close() // Closed earlier-than-planned („planned” by the defer statement above), so we can re-open with O_WRONLY (as opposed to O_RDONLY, which is what os.Open uses)
			}
			if to_wrt, err := deepOpen(args[2], os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0775); err != nil {
				return err
			} else {
				defer to_wrt.Close()
				return process(content, using, to_wrt)
			}
		}

		// Multi-content case
		return filepath.Walk(args[0], func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.IsDir() {
				return nil
			}

			if content_new, err := os.Open(path); err != nil {
				return err
			} else if subpath, success := strings.CutPrefix(path, args[0]); !success {
				content_new.Close()
				return errors.New("could not get subpath from path \"" + path + "\" with base \"" + args[0] + "\"")
			} else if to_new, err := deepOpen(args[2]+string(os.PathSeparator)+subpath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0775); err != nil {
				println("Got subpath \"" + subpath + "\" from path \"" + path + "\" with base \"" + args[0] + "\". This may help debug the error below:")
				content_new.Close()
				return err
			} else {
				defer content_new.Close()
				defer to_new.Close()
				return process(content_new, using, to_new)
			}
		})
	},
}

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// Here you will define your flags and configuration settings.
	// Cobra supports persistent flags, which, if defined here,
	// will be global for your application.

	// rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file (default is $HOME/.guziohub-generator.yaml)")

	// Cobra also supports local flags, which will only run
	// when this action is called directly.
	//rootCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}

func deepOpen(name string, flag int, perm os.FileMode) (*os.File, error) {
	if err := os.MkdirAll(filepath.Dir(name), perm); err != nil {
		return nil, err
	}
	return os.OpenFile(name, flag, perm)
}

func process(content *os.File, using *os.File, to *os.File) error {
	println("Would've processed \"" + content.Name() + "\" using template \"" + using.Name() + "\" to output \"" + to.Name() + "\"")
	return nil
}