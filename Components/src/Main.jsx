const contract = props.contract || "chat-simple.ws-protocol-63";

State.init({
  bootstraping: true,
  loggedIn: false,
  channelList: [],
  selectedChannel: 0,
  usersList: [],
  chatMessages: [],
  message: "",
  inputId: 0,
});

// DATA FETCHING FUNCTIONS - VIEW CALLS
const updateMemberList = () =>
  Near.asyncCalimeroView(contract, "get_members").then((m) => {
    State.update({ usersList: m });
    return m;
  });

const updateChannelList = () =>
  Near.asyncCalimeroView(
    contract,
    "get_groups",
    { account: context.accountId },
    undefined,
    true
  ).then((c) => State.update({ channelList: c }));

const setMessages = () => {
  if (state.channelList[0]) {
    Near.asyncCalimeroView(
      contract,
      "get_messages",
      {
        group: state.channelList[0],
      },
      undefined,
      true
    ).then((m) => {
      State.update({ chatMessages: m });
    });
  }
};

// HELPER FUNCTIONS - CHANGE DATA OR CHANGE FUNCTIONS
const onChangeMessage = ({ target }) => {
  State.update({ message: target.value });
};

const updateInputId = (id) => {
  State.update({ inputId: id });
};
const sendMessage = () => {
  if (!state.message) {
    return;
  }
  let params = {};
  params = { group: state.channelList[0] };
  params.message = state.message;
  params.timestamp = Date.now();
  State.update({ message: "" });
  updateInputId(Math.random().toString(36));
  Near.fakCalimeroCall(contract, "send_message", params);
  setMessages();
};

// CALIMERO FUNCTION ACCESSKEYS FUNCTIONS
const joinCurb = () => {
  Near.requestCalimeroFak(contract);
};

const isMember = (accountId, members) => {
  return (members || state.usersList)
    .map((user) => user.id)
    .includes(accountId);
};

const verifyKey = () => {
  Near.hasValidCalimeroFak(contract).then((result) => {
    State.update({ bootstraping: false, loggedIn: result });
    if (result) {
      updateMemberList().then((members) => {
        if (!isMember(context.accountId, members)) {
          Near.fakCalimeroCall(contract, "join");
        }
      });
      updateChannelList();
    }
  });
};

if (state.bootstraping) {
  verifyKey();
}

updateMemberList();
updateChannelList();
setMessages();

const Separator = styled.div`
  height: 1px;
  padding: 1px;
  width: 3rem;
  background-color: #111;
`;

const Message = styled.div`
  margin-bottom: 1rem;
  background-color: #5c5470;
  width: 100%;
  padding: 2px;
`;

const formatTimeAgo = (seconds) => {
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const weeks = Math.floor(days / 7);
  const months = Math.floor(weeks / 4);

  if (months > 0) {
    return `${months} month${months > 1 ? "s" : ""} ago`;
  } else if (weeks > 0) {
    return `${weeks} week${weeks > 1 ? "s" : ""} ago`;
  } else if (days > 0) {
    return `${days} day${days > 1 ? "s" : ""} ago`;
  } else if (hours > 0) {
    return `${hours} hour${hours > 1 ? "s" : ""} ago`;
  } else if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? "s" : ""} ago`;
  } else {
    return `just now`;
  }
};

const MessageData = styled.div`
  display: flex;
  column-gap: 1rem;
  margin-top: 0.5rem;
  margin-bottom: 0.5rem;
`;

const MainContainer = styled.div`
  background-color: #352f44;
  padding: 1rem;
  width: 100%;
`;

const SenderText = styled.div`
  color: #faf0e6;
  font-size: 1rem;
  line-height: 1.25rem;
  font-weight: 700;
`;
const TimeAgo = styled.div`
  color: #b9b4c7;
  font-size: 0.75rem;
  line-height: 1rem;
  margin-top: 2px;
`;

const MessageText = styled.div`
  color: #fff;
  margin-top: 1rem;
  margin-bottom: 1rem;
`;

const Title = styled.div`
  color: #fff;
  font-size: 1.25rem;
  line-height: 1.5rem;
  font-weight: 700;
`;

const IconSend = styled.i`
  margin-top: 0.3rem;
  margin-left: 1rem;
  font-size: 1.25rem;
  cursor: pointer;
  color: #797978;
  :hover {
    color: #fff;
  }
`;

const LoginText = styled.div`
  color: #fff;
`;

const ButtonJoin = styled.div`
  width: 100px;
  padding: 4px;
  background-color: #b9b4c7;
  color: #111;
  font-size: 1.25rem;
  line-height: 1.5rem;
  font-weight: 700;
  border-radius: 8px;
`;

return (
  <MainContainer>
    {context.accountId ? (
      <>
        {state.bootstraping ? (
          <Title>Loading...</Title>
        ) : (
          <>
            {state.loggedIn && isMember(context.accountId) ? (
              <div>
                <Title>Calimero Chat - NEAR APAC</Title>
                {state.chatMessages.length === 0 && (
                  <Title>No messages yet</Title>
                )}
                <div>
                  {state.chatMessages.map((message, id) => (
                    <Message key={id}>
                      <MessageData>
                        <Widget
                          src="fran-cali.testnet/widget/UserProfileIcon"
                          props={{
                            accountId: props.message.sender,
                            showStatus: false,
                          }}
                        />
                        <SenderText>{message.sender}</SenderText>
                        <TimeAgo>
                          {formatTimeAgo(
                            (Date.now() - message.timestamp) / 1000
                          )}
                        </TimeAgo>
                      </MessageData>

                      <MessageText>{message.text}</MessageText>
                    </Message>
                  ))}
                </div>
                <div className="d-flex gap-x-2">
                  <input
                    onChange={onChangeMessage}
                    onKeyUp={(e) => {
                      if (e.key == "Enter") {
                        sendMessage();
                      }
                    }}
                    placeholder={"send a message"}
                    key={state.inputId}
                    value={state.message}
                    autoFocus
                  />
                  <IconSend
                    className="bi bi-send-fill"
                    onClick={() => sendMessage()}
                  ></IconSend>
                </div>
              </div>
            ) : (
              <ButtonJoin onClick={joinCurb}>Join Chat</ButtonJoin>
            )}
          </>
        )}
      </>
    ) : (
      <Title>Please login to continue</Title>
    )}
  </MainContainer>
);
