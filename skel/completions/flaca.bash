_basher__flaca() {
	local i cur prev opts cmd
	COMPREPLY=()
	cur="${COMP_WORDS[COMP_CWORD]}"
	prev="${COMP_WORDS[COMP_CWORD-1]}"
	cmd=""
	opts=()

	for i in ${COMP_WORDS[@]}; do
		case "${i}" in
			flaca)
				cmd="flaca"
				;;

			*)
				;;
		esac
	done

	if [ ! -z "${cmd}" ]; then
		opts=()
		
		if [[ ! " ${COMP_LINE} " =~ " -h " ]] && [[ ! " ${COMP_LINE} " =~ " --help " ]]; then
			opts+=("-h")
			opts+=("--help")
		fi
		if [[ ! " ${COMP_LINE} " =~ " -p " ]] && [[ ! " ${COMP_LINE} " =~ " --progress " ]]; then
			opts+=("-p")
			opts+=("--progress")
		fi
		if [[ ! " ${COMP_LINE} " =~ " -V " ]] && [[ ! " ${COMP_LINE} " =~ " --version " ]]; then
			opts+=("-V")
			opts+=("--version")
		fi
		if [[ ! " ${COMP_LINE} " =~ " -l " ]] && [[ ! " ${COMP_LINE} " =~ " --list " ]]; then
			opts+=("-l")
			opts+=("--list")
		fi

		opts=" ${opts[@]} "
		if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
			COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
			return 0
		fi

		case "${prev}" in
			-l|--list)
				COMPREPLY=( $( compgen -f "${cur}" ) )
				return 0
				;;
			*)
				COMPREPLY=()
				;;
		esac

		COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
		return 0
	fi
}

complete -F _basher__flaca -o bashdefault -o default flaca
