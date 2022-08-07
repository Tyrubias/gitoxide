#!/bin/bash
set -eu -o pipefail

git init;

function baseline() {
    local kind=$1
    local refspec=$2

  cat <<EOF >.git/config
[remote "test"]
  url = .
  $kind = "$refspec"
EOF

    git ls-remote "test" && status=0 || status=$?
    {
        echo "$kind" "$refspec"
        echo "$status"
    } >> baseline.git
}


# invalid

baseline push ''
baseline push '::'
baseline fetch '::'
baseline fetch '^a:'
baseline fetch '^a:b'
baseline fetch '^:'
baseline fetch '^:b'
baseline fetch '^'
baseline push '^'

baseline fetch '^refs/heads/qa/*/*'
baseline push '^refs/heads/qa/*/*'

baseline push 'refs/heads/*:refs/remotes/frotz'
baseline push 'refs/heads:refs/remotes/frotz/*'

baseline fetch 'refs/heads/*:refs/remotes/frotz'
baseline fetch 'refs/heads:refs/remotes/frotz/*'
baseline fetch 'refs/heads/main::refs/remotes/frotz/xyzzy'
baseline fetch 'refs/heads/maste :refs/remotes/frotz/xyzzy'
baseline fetch 'main~1:refs/remotes/frotz/backup'
baseline fetch 'HEAD~4:refs/remotes/frotz/new'
baseline push 'refs/heads/ nitfol'
baseline fetch 'refs/heads/ nitfol'
baseline push 'HEAD:'
baseline push 'refs/heads/ nitfol:'
baseline fetch 'refs/heads/ nitfol:'
baseline push ':refs/remotes/frotz/delete me'
baseline fetch ':refs/remotes/frotz/HEAD to me'
baseline fetch 'refs/heads/*/*/for-linus:refs/remotes/mine/*'
baseline push 'refs/heads/*/*/for-linus:refs/remotes/mine/*'

baseline fetch 'refs/heads/*g*/for-linus:refs/remotes/mine/*'
baseline push 'refs/heads/*g*/for-linus:refs/remotes/mine/*'
bad=$(printf '\011tab')
baseline fetch "refs/heads/${bad}"

# valid
baseline push '+:'
baseline push ':'

baseline fetch ''
baseline fetch ':'
baseline fetch '+'
baseline push 'refs/heads/main:refs/remotes/frotz/xyzzy'
baseline push 'refs/heads/*:refs/remotes/frotz/*'


baseline fetch 'refs/heads/*:refs/remotes/frotz/*'
baseline fetch 'refs/heads/main:refs/remotes/frotz/xyzzy'

baseline push 'main~1:refs/remotes/frotz/backup'
baseline push 'HEAD~4:refs/remotes/frotz/new'

baseline push 'HEAD'
baseline fetch 'HEAD'
baseline push '@'
baseline fetch '@'

baseline push '^@'
baseline fetch '^@'

baseline push '+@'
baseline fetch '+@'

baseline fetch 'HEAD:'

baseline push ':refs/remotes/frotz/deleteme'
baseline fetch ':refs/remotes/frotz/HEAD-to-me'

baseline push ':a'
baseline push '+:a'

baseline fetch ':a'
baseline fetch '+:a'

baseline fetch 'refs/heads/*/for-linus:refs/remotes/mine/*-blah'
baseline push 'refs/heads/*/for-linus:refs/remotes/mine/*-blah'

baseline fetch 'refs/heads*/for-linus:refs/remotes/mine/*'
baseline push 'refs/heads*/for-linus:refs/remotes/mine/*'


baseline fetch 'refs/heads/*/for-linus:refs/remotes/mine/*'
baseline push 'refs/heads/*/for-linus:refs/remotes/mine/*'

good=$(printf '\303\204')
baseline fetch "refs/heads/${good}"
