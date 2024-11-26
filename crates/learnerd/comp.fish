# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_learner_global_optspecs
	string join \n v/verbose p/path= accept-defaults g/generate= h/help V/version
end

function __fish_learner_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_learner_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_learner_using_subcommand
	set -l cmd (__fish_learner_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c learner -n "__fish_learner_needs_command" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_needs_command" -s g -l generate -d 'Generate shell completions, using doc strings for subcommand hints' -r -f -a "{bash\t'',elvish\t'',fish\t'',powershell\t'',zsh\t''}"
complete -c learner -n "__fish_learner_needs_command" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_needs_command" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_needs_command" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_needs_command" -s V -l version -d 'Print version'
complete -c learner -n "__fish_learner_needs_command" -f -a "init" -d 'Initialize a new learner database'
complete -c learner -n "__fish_learner_needs_command" -f -a "add" -d 'Add a paper to the database by its identifier'
complete -c learner -n "__fish_learner_needs_command" -f -a "remove" -d 'Remove papers from the database'
complete -c learner -n "__fish_learner_needs_command" -f -a "search" -d 'Search for papers in the database'
complete -c learner -n "__fish_learner_needs_command" -f -a "daemon" -d 'Manage the learnerd daemon'
complete -c learner -n "__fish_learner_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c learner -n "__fish_learner_using_subcommand init" -l db-path -d 'Path to use to store [`Database`]. Defaults to [`Database::default_path`]' -r -F
complete -c learner -n "__fish_learner_using_subcommand init" -l storage-path -d 'Path to use to store documents. Defaults to [`Database::default_storage_path`]' -r -F
complete -c learner -n "__fish_learner_using_subcommand init" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand init" -l default-retrievers -d 'Whether to use the default set of retrievier configurations. Defaults to `true`'
complete -c learner -n "__fish_learner_using_subcommand init" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand init" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand init" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_using_subcommand add" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand add" -l pdf -d 'Force PDF download'
complete -c learner -n "__fish_learner_using_subcommand add" -l no-pdf -d 'Skip PDF download'
complete -c learner -n "__fish_learner_using_subcommand add" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand add" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand add" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_using_subcommand remove" -l author -d 'Filter by author name' -r
complete -c learner -n "__fish_learner_using_subcommand remove" -l source -d 'Filter by paper source (arxiv, doi, iacr)' -r
complete -c learner -n "__fish_learner_using_subcommand remove" -l before -d 'Filter by publication date (YYYY-MM-DD)' -r
complete -c learner -n "__fish_learner_using_subcommand remove" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand remove" -l dry-run -d 'Show what would be removed without actually removing'
complete -c learner -n "__fish_learner_using_subcommand remove" -l force -d 'Skip confirmation prompts'
complete -c learner -n "__fish_learner_using_subcommand remove" -l remove-pdf -d 'Remove associated PDFs'
complete -c learner -n "__fish_learner_using_subcommand remove" -l keep-pdf -d 'Keep PDFs when removing papers'
complete -c learner -n "__fish_learner_using_subcommand remove" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand remove" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand remove" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_using_subcommand search" -l author -d 'Filter by author name' -r
complete -c learner -n "__fish_learner_using_subcommand search" -l source -d 'Filter by paper source (arxiv, doi, iacr)' -r
complete -c learner -n "__fish_learner_using_subcommand search" -l before -d 'Filter by publication date (YYYY-MM-DD)' -r
complete -c learner -n "__fish_learner_using_subcommand search" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand search" -l detailed -d 'Show detailed paper information'
complete -c learner -n "__fish_learner_using_subcommand search" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand search" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand search" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -s h -l help -d 'Print help'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "start" -d 'Start the daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "stop" -d 'Stop a running daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "restart" -d 'Restart the daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "install" -d 'Install the daemon as a system service'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "uninstall" -d 'Remove the daemon from system services'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "status" -d 'Display current daemon status'
complete -c learner -n "__fish_learner_using_subcommand daemon; and not __fish_seen_subcommand_from start stop restart install uninstall status help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from start" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from start" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from start" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from start" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from stop" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from stop" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from stop" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from stop" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from restart" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from restart" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from restart" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from restart" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from install" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from install" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from install" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from install" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from uninstall" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from uninstall" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from uninstall" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from uninstall" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from status" -s p -l path -d 'Path to the database file. This is where the database will be created or referenced from. If not specified, uses the default platform-specific data directory' -r -F
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from status" -s v -l verbose -d 'Increase logging verbosity'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from status" -l accept-defaults -d 'Skip all prompts and accept defaults (mostly for testing)'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "start" -d 'Start the daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "stop" -d 'Stop a running daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "restart" -d 'Restart the daemon process'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "install" -d 'Install the daemon as a system service'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "uninstall" -d 'Remove the daemon from system services'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "status" -d 'Display current daemon status'
complete -c learner -n "__fish_learner_using_subcommand daemon; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "init" -d 'Initialize a new learner database'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "add" -d 'Add a paper to the database by its identifier'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "remove" -d 'Remove papers from the database'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "search" -d 'Search for papers in the database'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "daemon" -d 'Manage the learnerd daemon'
complete -c learner -n "__fish_learner_using_subcommand help; and not __fish_seen_subcommand_from init add remove search daemon help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "start" -d 'Start the daemon process'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "stop" -d 'Stop a running daemon process'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "restart" -d 'Restart the daemon process'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "install" -d 'Install the daemon as a system service'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "uninstall" -d 'Remove the daemon from system services'
complete -c learner -n "__fish_learner_using_subcommand help; and __fish_seen_subcommand_from daemon" -f -a "status" -d 'Display current daemon status'
